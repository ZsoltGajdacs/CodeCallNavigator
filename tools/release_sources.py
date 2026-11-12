"""Preserve corresponding source and bind it to the packaged build.

Local packaging may use a dirty tree. Public source releases require committed
inputs, matching package manifests and verified upstream source archives.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

from bootstrap import checkout_patch


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git(root, *arguments):
    return subprocess.check_output(["git", "-C", str(root), *arguments])


def source_identity(root):
    return {"commit": git(root, "rev-parse", "HEAD").decode().strip(),
            "patch_sha256": hashlib.sha256(checkout_patch(["git", "-C", str(root)])).hexdigest()}


def require_clean_source(root):
    if checkout_patch(["git", "-C", str(root)]):
        raise RuntimeError("Commit project changes and remove unrelated untracked files before a public release")


def archive_project(root, destination):
    require_clean_source(root)
    subprocess.run(["git", "-C", str(root), "archive", "--format=tar", "HEAD", "-o", str(destination)], check=True)


def bundle_repository(repository, destination):
    # A bundle preserves the upstream commit identity needed by bootstrap.
    subprocess.run(["git", "-C", str(repository), "bundle", "create", str(destination), "HEAD"], check=True)


def verified_archive(directory, specification):
    filename = specification["filename"]
    if Path(filename).name != filename:
        raise RuntimeError("Source archive filename must be a basename")
    archive = directory / filename
    if not archive.is_file():
        raise RuntimeError(f"Missing corresponding source: {archive}; obtain {specification.get('url', filename)}")
    if digest(archive) != specification["sha256"]:
        raise RuntimeError(f"Source archive checksum mismatch: {filename}")
    return archive


def package_files(resources):
    base = resources.parent
    # Bind all executable/resource bytes, including libraries, scripts and notices.
    return sorted(path for path in base.rglob("*") if path.is_file()
                  and path.name != "BUILD_SOURCE.json" and "_CodeSignature" not in path.parts)


def build_manifest_path(resources):
    # A manifest written inside a signed app would invalidate its resource seal.
    if resources.parent.name == "Contents":
        return resources.parents[2] / (resources.parents[1].name + ".build.json")
    return resources / "BUILD_SOURCE.json"


def write_build_manifest(root, resources, files=None):
    files = package_files(resources) if files is None else files
    manifest = {"source": source_identity(root),
                "files": {str(path.relative_to(resources.parent)): digest(path) for path in files}}
    build_manifest_path(resources).write_text(json.dumps(manifest, indent=2) + "\n")


def verify_build_manifest(root, resources):
    manifest = json.loads(build_manifest_path(resources).read_text())
    if manifest["source"] != source_identity(root):
        raise RuntimeError("Packaged build does not match the current project source; rebuild it")
    actual_files = {str(path.relative_to(resources.parent)): digest(path) for path in package_files(resources)}
    if actual_files != manifest["files"]:
        raise RuntimeError("Packaged files changed since the build; rebuild before releasing")


def copy_native_sources(resources, source_cache, destination, specifications):
    inventory = resources / "NATIVE_SOURCES.json"
    if not inventory.is_file():
        return
    for name, installed in json.loads(inventory.read_text()).items():
        specification = specifications[name]
        if installed["version"] != specification["version"]:
            raise RuntimeError(f"Unreviewed native source version for {name}: {installed['version']}")
        shutil.copy2(verified_archive(source_cache, specification), destination)
    shutil.copy2(inventory, destination)


def verify_aviator_version(resources, specification):
    jars = sorted((resources / "analyzers/java-bytecode/lib").glob("aviator-*.jar"))
    if [path.name for path in jars] != [f"aviator-{specification['version']}.jar"]:
        raise RuntimeError("Packaged Aviator version does not match the reviewed corresponding source")


def prepare_source_release(root, resources_list, output):
    """Create a source archive only after validating the exact packaged inputs."""
    require_clean_source(root)
    identity = source_identity(root)
    for resources in resources_list:
        verify_build_manifest(root, resources)
    specifications = json.loads((root / "tools/release-sources.lock.json").read_text())
    for resources in resources_list:
        verify_aviator_version(resources, specifications["aviator"])
    source_cache = root / "vendor/release-sources"
    with tempfile.TemporaryDirectory(prefix="codecallnavigator-source-", dir=output) as temporary:
        staging = Path(temporary) / "CodeCallNavigator-source"
        staging.mkdir()
        archive_project(root, staging / "project.tar")
        bundle_repository(root, staging / "project.bundle")
        upstreams = staging / "upstreams"
        upstreams.mkdir()
        for name in ("callatlas", "rune", "java-callgraph2"):
            bundle_repository(root / "vendor" / name, upstreams / f"{name}.bundle")
        rust = staging / "cargo-vendor"
        result = subprocess.run(["cargo", "vendor", "--locked", "--offline", "--versioned-dirs", str(rust)],
                                cwd=root, check=True, capture_output=True, text=True)
        (staging / "cargo-config.toml").write_text(result.stdout.replace(str(rust), "../cargo-vendor"))
        dependencies = staging / "dependency-sources"
        dependencies.mkdir()
        shutil.copy2(verified_archive(source_cache, specifications["aviator"]), dependencies)
        builds = staging / "builds"
        builds.mkdir()
        for index, resources in enumerate(resources_list):
            copy_native_sources(resources, source_cache, dependencies, specifications)
            shutil.copy2(build_manifest_path(resources), builds / f"{index}.json")
        shutil.copy2(root / "docs/public-releases.md", staging / "REBUILD.md")
        manifest = {"source": identity,
                    "files": {str(path.relative_to(staging)): digest(path)
                              for path in sorted(staging.rglob("*")) if path.is_file()}}
        (staging / "SOURCE_MANIFEST.json").write_text(json.dumps(manifest, indent=2) + "\n")
        require_clean_source(root)
        if source_identity(root) != identity:
            raise RuntimeError("Project source changed while preparing the release")
        destination = output / "CodeCallNavigator-source.tar.gz"
        candidate = Path(temporary) / destination.name
        with tarfile.open(candidate, "w:gz", compresslevel=1) as archive:
            archive.add(staging, arcname=staging.name)
        candidate.replace(destination)
    return destination


def write_release_checksums(output, artifacts):
    lines = [f"{digest(path)}  {path.name}\n" for path in artifacts]
    (output / "SHA256SUMS").write_text("".join(lines))
