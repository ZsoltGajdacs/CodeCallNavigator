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
import tempfile
import time
import zipfile
from bootstrap import checkout_patch

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
    relocate_graphviz(copied)
    environment = {**os.environ, "GVBINDIR": str(destination / "lib" / "graphviz"), "PATH": "/usr/bin:/bin"}
    dot = destination / "bin" / "dot"
    run(dot, "-c", env=environment)
    output = subprocess.check_output([str(dot), "-Tjson"], input=b'digraph{node[label=""];a->b}', env=environment)
    if len(json.loads(output)["objects"]) != 2:
        raise RuntimeError("Bundled Graphviz smoke failed")
    return Path(executable).resolve().parent.parent


def dependency_notices(resources, graphviz_prefix=None):
    notices = resources / "Licenses"
    notices.mkdir()
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
    package_root = Path.home() / ".nuget" / "packages"
    for package in deps["libraries"]:
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
        if package["name"] == "codegraph":
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
        return
    for source in directory.rglob("*"):
        if notice_file(source):
            destination = target / source.relative_to(directory)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)


def notice_file(source):
    name = source.name.lower()
    relevant = name.startswith(("license", "notice", "copyright")) or name.endswith(".nuspec")
    return source.is_file() and relevant


def copy_analyzers(resources):
    analyzers = resources / "analyzers"
    analyzers.mkdir()
    copy_tree(ROOT / "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net10.0", analyzers / "callatlas")
    copy_tree(ROOT / "tools/java-bytecode/build/install/codegraph-bytecode", analyzers / "java-bytecode")
    copy_tree(ROOT / "tools/typescript", analyzers / "typescript")
    shutil.copy2(ROOT / "tools/gradle-metadata.gradle", analyzers)
    shutil.copy2(ROOT / "tools/maven-metadata/build/libs/codegraph-maven-metadata.jar",
                 analyzers / "maven-metadata.jar")
    shutil.copy2(ROOT / "analyzers.lock.json", resources)
    return analyzers


def make_engine(destination):
    binary = destination / "bin"
    resources = destination / "Resources"
    binary.mkdir(parents=True)
    resources.mkdir()
    shutil.copy2(ROOT / "target/release/codegraph-engine", binary)
    copy_analyzers(resources)
    dependency_notices(resources)


def make_app(app):
    contents = app / "Contents"
    resources = contents / "Resources"
    (contents / "MacOS").mkdir(parents=True)
    resources.mkdir()
    shutil.copy2(ROOT / "target/release/codegraph", contents / "MacOS/CodeGraph")
    shutil.copy2(ROOT / "target/release/codegraph-engine", contents / "MacOS/codegraph-engine")
    analyzers = copy_analyzers(resources)
    graphviz_prefix = bundle_graphviz(analyzers / "graphviz")
    dependency_notices(resources, graphviz_prefix)
    with (contents / "Info.plist").open("wb") as stream:
        plistlib.dump({"CFBundleIdentifier": "dev.codegraph.app", "CFBundleName": "CodeGraph",
            "CFBundleExecutable": "CodeGraph", "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": "0.1.0", "CFBundleVersion": "1",
            "NSHighResolutionCapable": True, "LSMinimumSystemVersion": "27.0"}, stream)
    run("/usr/bin/codesign", "--force", "--deep", "--sign", "-", app)
    run("/usr/bin/codesign", "--verify", "--deep", "--strict", app)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine-only', action='store_true', help='Package engine without desktop or Graphviz')
    args = parser.parse_args()
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise SystemExit("This package targets local Apple Silicon macOS")
    verify_dependency_sources()
    run("cargo", "build", "--release", "--locked", "-p", "codegraph-engine", "--bin", "codegraph-engine", cwd=ROOT)
    output = ROOT / "dist"
    output.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="codegraph-engine-package-", dir=output) as temporary:
        engine = Path(temporary) / "CodeGraphEngine"
        make_engine(engine)
        destination = output / "CodeGraphEngine"
        if destination.exists():
            shutil.rmtree(destination)
        engine.rename(destination)
    if args.engine_only:
        return
    run("cargo", "build", "--release", "--locked", "--bin", "codegraph", cwd=ROOT)
    with tempfile.TemporaryDirectory(prefix="codegraph-package-", dir=output) as temporary:
        app = Path(temporary) / "CodeGraph.app"
        make_app(app)
        destination = output / "CodeGraph.app"
        if destination.exists():
            destination.rename(output / time.strftime("CodeGraph.previous-%Y%m%d-%H%M%S.app"))
        app.rename(destination)
    run("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", destination, output / "CodeGraph.zip")
    print(destination)


if __name__ == "__main__":
    main()
