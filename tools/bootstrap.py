#!/usr/bin/env python3
"""Fetch pinned public dependencies without installing any SDKs."""
import hashlib
import argparse
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def fetch_repository(name, spec):
    destination = ROOT / "vendor" / name
    if not destination.exists():
        subprocess.run(["git", "clone", spec["repository"], str(destination)], check=True)
    revision = subprocess.check_output(["git", "-C", str(destination), "rev-parse", "HEAD"], text=True).strip()
    if revision != spec["revision"]:
        dirty = subprocess.check_output(["git", "-C", str(destination), "status", "--porcelain"], text=True)
        if dirty:
            raise RuntimeError(f"Refusing to overwrite changes in {destination}")
    subprocess.run(["git", "-C", str(destination), "checkout", "--detach", spec["revision"]], check=True)


def verified_patch(spec, prefix=""):
    patch = ROOT / spec[f"{prefix}patch"]
    if hashlib.sha256(patch.read_bytes()).hexdigest() != spec[f"{prefix}patch_sha256"]:
        raise RuntimeError(f"Patch checksum mismatch: {patch}")
    return patch


def apply_patch(spec, name="callatlas"):
    patch = verified_patch(spec)
    repository = ROOT / "vendor" / name
    command = ["git", "-C", str(repository)]
    difference = checkout_patch(command)
    if difference == patch.read_bytes():
        return
    if not difference:
        subprocess.run([*command, "apply", str(patch)], check=True)
        return
    upgrade_legacy_patch(command, spec, difference, patch)


def checkout_patch(command):
    untracked = subprocess.check_output([*command, "ls-files", "--others", "--exclude-standard", "-z"])
    environment = {**os.environ, "GIT_OPTIONAL_LOCKS": "0"}
    if not untracked:
        return subprocess.check_output([*command, "diff", "--binary", "HEAD"], env=environment)
    # git apply leaves newly added source files untracked. Include them in the
    # exact-patch comparison without modifying the real index or staging work.
    # Unrelated files make the complete diff differ and are rejected by callers.
    with tempfile.TemporaryDirectory(prefix="codegraph-patch-index-") as temporary:
        original_index = subprocess.check_output([*command, "rev-parse", "--path-format=absolute", "--git-path", "index"],
                                                 env=environment, text=True).strip()
        temporary_index = Path(temporary) / "index"
        shutil.copyfile(original_index, temporary_index)
        environment.update(GIT_INDEX_FILE=str(temporary_index), GIT_LITERAL_PATHSPECS="1")
        subprocess.run([*command, "add", "--intent-to-add", "--pathspec-from-file=-", "--pathspec-file-nul"],
                       input=untracked, env=environment, check=True)
        return subprocess.check_output([*command, "diff", "--binary", "HEAD"], env=environment)


def upgrade_legacy_patch(command, spec, difference, patch):
    candidates = [verified_patch(spec, prefix) for prefix in ("pre_visibility_", "previous_", "older_", "legacy_")
                  if f"{prefix}patch" in spec]
    legacy = next((candidate for candidate in candidates if difference == candidate.read_bytes()), None)
    if legacy is None:
        raise RuntimeError(f"Dependency source differs from the supported patch (tracked edits or unrelated untracked files); inspect {command[-1]}")
    # Upgrade only the exact previously shipped patch. git apply checks every hunk
    # before writing; restore the legacy patch if applying its replacement fails.
    subprocess.run([*command, "apply", "--reverse", str(legacy)], check=True)
    try:
        subprocess.run([*command, "apply", str(patch)], check=True)
    except subprocess.CalledProcessError:
        subprocess.run([*command, "apply", str(legacy)], check=True)
        raise


def build_java_adapter():
    java_home = subprocess.check_output(["/usr/libexec/java_home", "-v", "25"], text=True).strip()
    environment = {**os.environ, "JAVA_HOME": java_home}
    subprocess.run([str(ROOT / "fixtures/gradle/gradlew"), "--project-dir", str(ROOT / "tools/java-bytecode"),
                    "installDist", "--console=plain", "--no-configuration-cache"], env=environment, check=True)
    subprocess.run([str(ROOT / "fixtures/gradle/gradlew"), "--project-dir", str(ROOT / "tools/maven-metadata"),
                    "jar", "--console=plain", "--no-configuration-cache"], env=environment, check=True)


def jdt_present(destination):
    launchers = list((destination / "plugins").glob("org.eclipse.equinox.launcher_*.jar"))
    return len(launchers) == 1 and (destination / "config_mac_arm" / "config.ini").is_file()


def download_verified(specification, destination):
    urllib.request.urlretrieve(specification["url"], destination)
    if hashlib.sha256(destination.read_bytes()).hexdigest() != specification["sha256"]:
        raise RuntimeError(f"Downloaded artifact checksum mismatch: {destination.name}")


def fetch_lombok(spec):
    destination = ROOT / "vendor" / f"lombok-{spec['version']}.jar"
    if destination.is_file() and hashlib.sha256(destination.read_bytes()).hexdigest() == spec["sha256"]:
        return
    with tempfile.TemporaryDirectory(prefix="codegraph-lombok-") as temporary:
        download = Path(temporary) / "lombok.jar"
        download_verified(spec, download)
        destination.write_bytes(download.read_bytes())


def fetch_jdt(spec):
    destination = ROOT / "vendor" / "jdtls"
    marker = destination / ".codegraph-sha256"
    if verified_jdt_cache(destination, spec["sha256"]):
        return
    if destination.exists() and any(destination.iterdir()):
        raise RuntimeError(f"Unverified existing JDT directory: {destination}; use a fresh vendor directory")
    with tempfile.TemporaryDirectory(prefix="codegraph-download-") as temporary:
        archive = Path(temporary) / "jdt.tar.gz"
        download_verified(spec, archive)
        destination.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive) as bundle:
            bundle.extractall(destination, filter="data")
        marker.write_text(spec["sha256"] + "\n")


def verified_jdt_cache(destination, checksum):
    marker = destination / ".codegraph-sha256"
    if not marker.is_file():
        return False
    return marker.read_text().strip() == checksum and jdt_present(destination)


def check_typescript_adapter():
    """Validate shipped bridge syntax without installing Node or project dependencies."""
    node = shutil.which("node")
    if node:
        for source in sorted((ROOT / "tools/typescript").glob("*.js")):
            subprocess.run([node, "--check", source], check=True)


def main():
    if sys.version_info < (3, 12):
        raise SystemExit("Dependency bootstrap requires Python 3.12+ for safe archive extraction.")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdt-investigation", action="store_true", help="Also fetch the rejected JDT/Lombok setup for reproducing its acceptance failure")
    args = parser.parse_args()
    lock = json.loads((ROOT / "analyzers.lock.json").read_text())
    (ROOT / "vendor").mkdir(exist_ok=True)
    for name in ("callatlas", "rune", "java-callgraph2"):
        fetch_repository(name, lock[name])
    apply_patch(lock["callatlas"])
    apply_patch(lock["rune"], "rune")
    if args.jdt_investigation:
        fetch_jdt(lock["jdtls"])
        fetch_lombok(lock["lombok"])
    subprocess.run(["dotnet", "build", str(ROOT / "vendor/callatlas/src/CallAtlas.Cli/CallAtlas.Cli.csproj"),
                    "-c", "Release"], check=True)
    build_java_adapter()
    check_typescript_adapter()


if __name__ == "__main__":
    main()
