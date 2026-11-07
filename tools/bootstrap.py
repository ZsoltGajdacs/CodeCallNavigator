#!/usr/bin/env python3
"""Fetch pinned public dependencies without installing any SDKs."""
import hashlib
import argparse
import json
import os
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


def apply_patch(spec):
    patch = ROOT / spec["patch"]
    if hashlib.sha256(patch.read_bytes()).hexdigest() != spec["patch_sha256"]:
        raise RuntimeError("CallAtlas identity patch checksum mismatch")
    command = ["git", "-C", str(ROOT / "vendor/callatlas"), "apply"]
    check = subprocess.run([*command, "--check", str(patch)], capture_output=True)
    if check.returncode == 0:
        subprocess.run([*command, str(patch)], check=True)
        return
    reverse = subprocess.run([*command, "--reverse", "--check", str(patch)], capture_output=True)
    if reverse.returncode != 0:
        raise RuntimeError("CallAtlas source differs from the supported identity patch; inspect vendor/callatlas")


def build_java_adapter():
    java_home = subprocess.check_output(["/usr/libexec/java_home", "-v", "25"], text=True).strip()
    environment = {**os.environ, "JAVA_HOME": java_home}
    subprocess.run([str(ROOT / "fixtures/gradle/gradlew"), "--project-dir", str(ROOT / "tools/java-bytecode"),
                    "installDist", "--console=plain", "--no-configuration-cache"], env=environment, check=True)


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
    if args.jdt_investigation:
        fetch_jdt(lock["jdtls"])
        fetch_lombok(lock["lombok"])
    subprocess.run(["dotnet", "build", str(ROOT / "vendor/callatlas/src/CallAtlas.Cli/CallAtlas.Cli.csproj"),
                    "-c", "Release"], check=True)
    build_java_adapter()


if __name__ == "__main__":
    main()
