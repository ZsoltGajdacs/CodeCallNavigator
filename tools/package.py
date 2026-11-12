#!/usr/bin/env python3
"""Build a local Apple Silicon app with relocatable analyzer/layout artifacts."""
from __future__ import annotations

import json
import argparse
import hashlib
import os
from pathlib import Path
import platform
import plistlib
import shutil
import subprocess
import sys
import tempfile
import time
import zipfile
import xml.etree.ElementTree as ElementTree
from bootstrap import checkout_patch
from release_sources import (prepare_source_release, require_clean_source,
                             source_identity, write_build_manifest, write_release_checksums)

ROOT = Path(__file__).resolve().parents[1]


def run(*arguments, **kwargs):
    kwargs.setdefault("check", True)
    return subprocess.run(list(map(str, arguments)), **kwargs)


def verify_dependency_sources():
    lock = json.loads((ROOT / "analyzers.lock.json").read_text())
    for name in ("callatlas", "rune", "java-callgraph2"):
        repository = ROOT / "vendor" / name
        revision = subprocess.check_output(["git", "-C", str(repository), "rev-parse", "HEAD"], text=True).strip()
        if revision != lock[name]["revision"]:
            raise RuntimeError(f"Unvalidated {name} revision; run tools/bootstrap.py")
        actual = checkout_patch(["git", "-C", str(repository)])
        expected = expected_source_patch(lock[name])
        if actual != expected:
            raise RuntimeError(f"Unvalidated local changes in {name}; inspect its source before packaging")


def expected_source_patch(specification):
    if "patch" not in specification:
        return b""
    patch_bytes = (ROOT / specification["patch"]).read_bytes()
    if hashlib.sha256(patch_bytes).hexdigest() != specification["patch_sha256"]:
        raise RuntimeError("Dependency source patch checksum mismatch")
    return patch_bytes


def copy_tree(source, destination):
    if not source.is_dir():
        raise RuntimeError(f"Missing build artifact: {source}")
    shutil.copytree(source, destination, symlinks=False)


def dependencies(binary):
    output = subprocess.check_output(["/usr/bin/otool", "-L", str(binary)], text=True)
    return [line.strip().split(" (", 1)[0] for line in output.splitlines()[1:]]


def brew_dependency(value):
    return value.startswith(("/opt/homebrew/", "/usr/local/"))


def graphviz_plugins(prefix, destination):
    plugins = prefix / "lib/graphviz"
    entries = []
    for name in ("core", "dot_layout"):
        choices = list(plugins.glob(f"libgvplugin_{name}.*.dylib"))
        if not choices:
            raise RuntimeError(f"Graphviz {name} plugin missing")
        plugin = choices[0].resolve()
        entries.append((plugin, destination / "lib" / "graphviz" / plugin.name))
    return entries


def collect_graphviz(source, destination):
    prefix = source.resolve().parent.parent
    entries = [(source.resolve(), destination / "bin/dot"), *graphviz_plugins(prefix, destination)]
    copied = {}
    while entries:
        original, target = entries.pop()
        if original in copied:
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(original, target)
        copied[original] = target
        required = (Path(item).resolve() for item in dependencies(original) if brew_dependency(item))
        entries.extend((path, destination / "lib" / path.name) for path in required)
    return copied


def relocate_graphviz(copied):
    for original, target in copied.items():
        run("/usr/bin/codesign", "--remove-signature", target, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
        for dependency in dependencies(original):
            if brew_dependency(dependency):
                replacement = copied[Path(dependency).resolve()]
                relative = os.path.relpath(replacement, target.parent)
                run("/usr/bin/install_name_tool", "-change", dependency, "@loader_path/" + relative, target)
        if target.suffix == ".dylib":
            run("/usr/bin/install_name_tool", "-id", "@rpath/" + target.name, target)
        run("/usr/bin/codesign", "--force", "--sign", "-", target, stderr=subprocess.DEVNULL)
        if any(brew_dependency(item) for item in dependencies(target)):
            raise RuntimeError(f"Graphviz still depends on Homebrew paths: {target}")


def bundle_graphviz(destination):
    executable = shutil.which("dot")
    if not executable:
        raise RuntimeError("Graphviz dot is required; install Graphviz before packaging")
    version = subprocess.run([executable, "-V"], capture_output=True, text=True, check=True).stderr
    expected = json.loads((ROOT / "analyzers.lock.json").read_text())["graphviz"]["version"]
    if f"version {expected} " not in version:
        raise RuntimeError(f"Expected validated Graphviz{expected}, got: {version.strip()}")
    copied = collect_graphviz(Path(executable), destination)
    native_source_inventory(copied, destination.parents[1])
    relocate_graphviz(copied)
    environment = {**os.environ, "GVBINDIR": str(destination / "lib" / "graphviz"), "PATH": "/usr/bin:/bin"}
    dot = destination / "bin" / "dot"
    run(dot, "-c", env=environment)
    output = subprocess.check_output([str(dot), "-Tjson"], input=b'digraph{node[label=""];a->b}', env=environment)
    if len(json.loads(output)["objects"]) != 2:
        raise RuntimeError("Bundled Graphviz smoke failed")
    return Path(executable).resolve().parent.parent


def native_source_inventory(copied, resources):
    inventory = {}
    for original in copied:
        name, source = installed_native_source(original)
        inventory[name] = source
    (resources / "NATIVE_SOURCES.json").write_text(json.dumps(inventory, indent=2) + "\n")


def installed_native_source(original):
    prefix = next((parent for parent in original.parents
                   if (parent / "INSTALL_RECEIPT.json").is_file()), None)
    if prefix is None:
        raise RuntimeError(f"Missing native dependency build receipt: {original}")
    name = prefix.parent.name
    if name not in ("graphviz", "libtool"):
        raise RuntimeError(f"Unreviewed native dependency {name}; add its license and source coverage")
    receipt = json.loads((prefix / "INSTALL_RECEIPT.json").read_text())
    formula = prefix / ".brew" / f"{name}.rb"
    return name, {"version": receipt["source"]["versions"]["stable"],
                  "formula": formula.read_text(), "receipt": receipt}


def dependency_notices(resources, graphviz_prefix=None):
    notices = resources / "Licenses"
    notices.mkdir()
    shutil.copy2(ROOT / "LICENSE", notices / "CodeCallNavigator-AGPL-3.0.txt")
    source_guide = (ROOT / "docs/public-releases.md").read_text()
    (notices / "SOURCE.md").write_text(source_guide.replace("(../THIRD_PARTY_NOTICES.md)", "(THIRD_PARTY_NOTICES.md)"))
    shutil.copy2(ROOT / "THIRD_PARTY_NOTICES.md", notices)
    for name in ("callatlas", "rune", "java-callgraph2"):
        shutil.copy2(ROOT / "vendor" / name / "LICENSE", notices / f"{name}-LICENSE")
    for font_notice in (ROOT / "vendor/rune/crates/rune/assets").glob("*-LICENSE.txt"):
        shutil.copy2(font_notice, notices)
    if graphviz_prefix:
        shutil.copy2(graphviz_prefix / "COPYING", notices / "graphviz-COPYING")
        shutil.copy2("/opt/homebrew/opt/libtool/share/libtool/COPYING.LIB", notices / "libltdl-COPYING.LIB")
    shutil.copy2(ROOT / "patches/callatlas-assembly-identity.patch", notices)
    copy_tree(ROOT / "tools/licenses", notices / "supplemental")
    jars = resources / "analyzers" / "java-bytecode" / "lib"
    for jar in jars.glob("*.jar"):
        copy_jar_notices(jar, notices)
    # Preserve NuGet license metadata for the copied managed assemblies.
    deps = json.loads((resources / "analyzers/callatlas/callatlas.deps.json").read_text())
    package_root = Path(os.environ.get("NUGET_PACKAGES", Path.home() / ".nuget/packages"))
    for package, specification in deps["libraries"].items():
        if specification["type"] == "project":
            continue
        name, version = package.rsplit("/", 1)
        directory = package_root / name.lower() / version.lower()
        copy_nuget_notices(directory, notices / "nuget" / name)
    rust_notices(notices / "rust")


def copy_jar_notices(jar, notices):
    with zipfile.ZipFile(jar) as archive:
        for entry in archive.namelist():
            base = Path(entry).name.upper()
            if base.startswith(("LICENSE", "NOTICE", "COPYING")) and not entry.endswith("/"):
                target = notices / jar.stem / entry.replace("/", "_")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(entry))


def rust_notices(destination):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"], cwd=ROOT))
    inventory = []
    for package in metadata["packages"]:
        if package["name"] in ("codecallnavigator", "codecallnavigator-core", "codecallnavigator-engine", "codecallnavigator-mcp"):
            continue
        directory = Path(package["manifest_path"]).parent
        target = destination / f"{package['name']}-{package['version']}"
        candidates = [path for path in directory.iterdir() if notice_file(path)]
        if package.get("license_file"):
            candidates.append(directory / package["license_file"])
        copy_crate_notices(candidates, target)
        inventory.append({"name": package["name"], "version": package["version"], "license": package.get("license")})
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "inventory.json").write_text(json.dumps(inventory, indent=2) + "\n")


def copy_crate_notices(candidates, destination):
    for source in set(candidates):
        if source.is_file():
            destination.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination / source.name)


def copy_nuget_notices(directory, target):
    if not directory.is_dir():
        raise RuntimeError(f"Missing NuGet license metadata: {directory}; restore packages before packaging")
    write_nuget_license(directory, target)
    for source in directory.rglob("*"):
        if notice_file(source):
            destination = target / source.relative_to(directory)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)


def write_nuget_license(directory, target):
    nuspecs = list(directory.glob("*.nuspec"))
    if len(nuspecs) != 1:
        raise RuntimeError(f"Expected one NuGet manifest in {directory}")
    metadata = ElementTree.parse(nuspecs[0]).getroot().find("{*}metadata")
    license = metadata.find("{*}license")
    if license is None:
        raise RuntimeError(f"Missing NuGet license in {nuspecs[0]}")
    if (license.attrib.get("type"), license.text) != ("expression", "MIT"):
        raise RuntimeError(f"Unreviewed NuGet license in {nuspecs[0]}; add its redistribution notices")
    copyright = metadata.findtext("{*}copyright", default="")
    authors = metadata.findtext("{*}authors", default="")
    permission = (ROOT / "tools/licenses/MIT.txt").read_text()
    target.mkdir(parents=True, exist_ok=True)
    (target / "LICENSE.txt").write_text(f"Package: {directory.parent.name} {directory.name}\nAuthors: {authors}\n{copyright}\n\n{permission}")


def notice_file(source):
    name = source.name.lower()
    normalized = name.replace("-", "").replace("_", "")
    relevant = normalized.startswith(("license", "notice", "copyright", "thirdpartynotice")) or name.endswith(".nuspec")
    return source.is_file() and relevant


def copy_analyzers(resources):
    verify_java_adapter(ROOT / "tools/java-bytecode/build/install/codecallnavigator-bytecode")
    analyzers = resources / "analyzers"
    analyzers.mkdir()
    copy_tree(ROOT / "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net10.0", analyzers / "callatlas")
    copy_tree(ROOT / "tools/java-bytecode/build/install/codecallnavigator-bytecode", analyzers / "java-bytecode")
    copy_tree(ROOT / "tools/typescript", analyzers / "typescript")
    shutil.copy2(ROOT / "tools/gradle-metadata.gradle", analyzers)
    shutil.copy2(ROOT / "tools/maven-metadata/build/libs/codecallnavigator-maven-metadata.jar",
                 analyzers / "maven-metadata.jar")
    shutil.copy2(ROOT / "analyzers.lock.json", resources)
    return analyzers


def verify_java_adapter(distribution):
    jar = distribution / "lib/codecallnavigator-bytecode.jar"
    package = "codecallnavigator/bytecode/"
    required = {package + name + ".class" for name in
                ("SourceInventory", "LambdaCorrelation", "LambdaCompilation",
                 "LambdaLowering", "LambdaSourceSite", "CompiledClassModel")}
    try:
        with zipfile.ZipFile(jar) as archive:
            complete = required.issubset(archive.namelist())
            current = complete and b"kind_v8" in archive.read(package + "SourceInventory.class")
    except (OSError, zipfile.BadZipFile) as error:
        raise RuntimeError("Java adapter is missing or invalid; rebuild matching analyzer resources") from error
    if not current:
        raise RuntimeError("Java adapter lacks kind_v8 compiled/source lambda correlation; rebuild matching analyzer resources")


def make_engine(destination):
    binary = destination / "bin"
    resources = destination / "Resources"
    binary.mkdir(parents=True)
    resources.mkdir()
    shutil.copy2(ROOT / "target/release/codecallnavigator-engine", binary)
    copy_analyzers(resources)
    dependency_notices(resources)
    write_build_manifest(ROOT, resources)


def make_app(app):
    contents = app / "Contents"
    resources = contents / "Resources"
    (contents / "MacOS").mkdir(parents=True)
    resources.mkdir()
    shutil.copy2(ROOT / "target/release/codecallnavigator", contents / "MacOS/CodeCallNavigator")
    shutil.copy2(ROOT / "target/release/codecallnavigator-engine", contents / "MacOS/codecallnavigator-engine")
    analyzers = copy_analyzers(resources)
    graphviz_prefix = bundle_graphviz(analyzers / "graphviz")
    dependency_notices(resources, graphviz_prefix)
    with (contents / "Info.plist").open("wb") as stream:
        plistlib.dump({"CFBundleIdentifier": "dev.codecallnavigator.app", "CFBundleName": "CodeCallNavigator",
            "CFBundleExecutable": "CodeCallNavigator", "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.1.0", "CFBundleVersion": "1",
            "NSHighResolutionCapable": True, "LSMinimumSystemVersion": "27.0"}, stream)
    (resources / "SOURCE_IDENTITY.json").write_text(json.dumps(source_identity(ROOT), indent=2) + "\n")
    run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", app)
    run("/usr/bin/codesign", "--verify", "--deep", "--strict", app)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine-only', action='store_true', help='Package engine without desktop or Graphviz')
    parser.add_argument('--release', action='store_true', help='Also preserve matching source; requires committed inputs and cached source archives')
    args = parser.parse_args()
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise SystemExit("This package targets local Apple Silicon macOS")
    if args.release:
        require_clean_source(ROOT)
        release_source = source_identity(ROOT)
        # Existing adapter outputs may predate a committed source edit. A release
        # must rebuild every helper, rather than trusting a capability marker.
        run(sys.executable, ROOT / "tools/bootstrap.py", cwd=ROOT)
    verify_dependency_sources()
    output = ROOT / "dist"
    output.mkdir(exist_ok=True)
    engine = build_engine(output)
    if args.engine_only:
        if args.release:
            finish_release(output, [engine / "Resources"], [], release_source)
        return
    app = build_app(output)
    if args.release:
        finish_release(output, [app / "Contents/Resources", engine / "Resources"],
                       [output / "CodeCallNavigator.zip"], release_source)
    print(app)


def build_engine(output):
    run("cargo", "build", "--release", "--locked", "-p", "codecallnavigator-engine", "--bin", "codecallnavigator-engine", cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix="codecallnavigator-engine-package-", dir=output) as temporary:
        engine = Path(temporary) / "CodeCallNavigatorEngine"
        make_engine(engine)
        destination = output / "CodeCallNavigatorEngine"
        if destination.exists():
            shutil.rmtree(destination)
        engine.rename(destination)
    return destination


def build_app(output):
    run("cargo", "build", "--release", "--locked", "--bin", "codecallnavigator", cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix="codecallnavigator-package-", dir=output) as temporary:
        app = Path(temporary) / "CodeCallNavigator.app"
        make_app(app)
        destination = output / "CodeCallNavigator.app"
        if destination.exists():
            destination.rename(output / time.strftime("CodeCallNavigator.previous-%Y%m%d-%H%M%S.app"))
        app.rename(destination)
    write_build_manifest(ROOT, destination / "Contents/Resources")
    run("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", destination, output / "CodeCallNavigator.zip")
    return destination


def finish_release(output, resources, artifacts, expected_source):
    if source_identity(ROOT) != expected_source:
        raise RuntimeError("Project source changed during the build; rebuild before releasing")
    verify_dependency_sources()
    source = prepare_source_release(ROOT, resources, output)
    engine = archive_engine(output)
    write_release_checksums(output, [*artifacts, engine, source])


def archive_engine(output):
    destination = output / "CodeCallNavigatorEngine.zip"
    # Cargo registry license files may carry epoch timestamps, outside ZIP's
    # representable date range. Clamp dates while retaining bytes and modes.
    with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED, strict_timestamps=False) as archive:
        for path in sorted((output / "CodeCallNavigatorEngine").rglob("*")):
            archive.write(path, path.relative_to(output))
    return destination


if __name__ == "__main__":
    main()
