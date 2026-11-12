#!/usr/bin/env python3
"""Provision exact native release inputs on a disposable GitHub-hosted Mac."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from bootstrap import download_verified
from release_sources import digest, verified_archive


def require_ci_runner():
    if (os.environ.get("GITHUB_ACTIONS"), os.environ.get("RUNNER_ENVIRONMENT")) != ("true", "github-hosted"):
        raise RuntimeError("Native provisioning is restricted to disposable GitHub-hosted runners")
    if (platform.system(), platform.machine()) != ("Darwin", "arm64"):
        raise RuntimeError("Native release inputs require an Apple Silicon macOS runner")
    if int(platform.mac_ver()[0].split(".")[0]) < 27:
        raise RuntimeError("Native release inputs require macOS 27 or newer")


def fetch_sources(root):
    specifications = json.loads((root / "tools/release-sources.lock.json").read_text())
    destination = root / "vendor/release-sources"
    destination.mkdir(parents=True, exist_ok=True)
    for specification in specifications.values():
        archive = destination / specification["filename"]
        if not archive.exists():
            with tempfile.TemporaryDirectory(prefix="source-", dir=destination) as temporary:
                candidate = Path(temporary) / archive.name
                download_verified(specification, candidate)
                candidate.replace(archive)
        verified_archive(destination, specification)


def fetch_bottle(name, specification, destination):
    query = urllib.parse.urlencode({"service": "ghcr.io", "scope": f"repository:homebrew/core/{name}:pull"})
    with urllib.request.urlopen("https://ghcr.io/token?" + query, timeout=60) as response:
        token = json.load(response)["token"]
    url = f"https://ghcr.io/v2/homebrew/core/{name}/blobs/sha256:{specification['sha256']}"
    request = urllib.request.Request(url, headers={"Authorization": "Bearer " + token})
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="bottle-", dir=destination.parent) as temporary:
        candidate = Path(temporary) / destination.name
        with urllib.request.urlopen(request, timeout=120) as response, candidate.open("wb") as output:
            shutil.copyfileobj(response, output)
        if digest(candidate) != specification["sha256"]:
            raise RuntimeError(f"Native bottle checksum mismatch: {name}")
        candidate.replace(destination)


def fetch_bottles(root, specifications):
    destination = root / "vendor/release-bottles"
    destination.mkdir(parents=True, exist_ok=True)
    for name, specification in specifications.items():
        archive = destination / specification["filename"]
        if not archive.exists():
            fetch_bottle(name, specification, archive)
        verified_archive(destination, specification)
    return destination


def brew(*arguments, check=True):
    environment = {**os.environ, "HOMEBREW_NO_AUTO_UPDATE": "1", "HOMEBREW_NO_INSTALL_CLEANUP": "1",
                   "HOMEBREW_NO_INSTALL_UPGRADE": "1", "HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK": "1"}
    result = subprocess.run(["brew", *map(str, arguments)], check=False, capture_output=True,
                            text=True, env=environment)
    if check and result.returncode:
        raise RuntimeError(f"Homebrew command failed: {' '.join(map(str, arguments))}\n{result.stdout}\n{result.stderr}")
    return result


def _remove_existing_package(name):
    if brew("list", "--versions", name, check=False).returncode == 0:
        brew("uninstall", "--ignore-dependencies", "--force", name)


def _update_homebrew():
    # The xcode-27 image initially shipped 7.0.12, before Homebrew fixed local
    # bottle dispatch. Refresh brew itself explicitly, without upgrading kegs.
    brew("update")
    output = brew("--version").stdout.strip()
    print(output, flush=True)
    version = re.match(r"Homebrew (\d+)\.(\d+)\.(\d+)", output)
    if version is None:
        raise RuntimeError(f"Cannot determine Homebrew version: {output}")
    if tuple(map(int, version.groups())) < (7, 0, 13):
        raise RuntimeError("Homebrew 7.0.13 or newer is required for pinned local bottle installation")


def _verify_package_version(name, specification):
    versions = brew("list", "--versions", name).stdout.strip().split()
    if versions != [name, specification["version"]]:
        raise RuntimeError(f"Unexpected installed native version for {name}: {' '.join(versions)}")


def install_bottles(specifications, directory):
    require_ci_runner()
    # Verify every input before removing either installed package. These names
    # intentionally retain the Cellar identities checked by the packager.
    archives = {name: verified_archive(directory, specification) for name, specification in specifications.items()}
    _update_homebrew()
    for name in ("graphviz", "libtool"):
        _remove_existing_package(name)
    for name in ("libtool", "graphviz"):
        brew("install", "--formula", archives[name])
    # Recheck both after dependency resolution, which must not upgrade libtool
    # while installing Graphviz from its original embedded formula.
    for name, specification in specifications.items():
        _verify_package_version(name, specification)


def validate_locks(root, specifications):
    sources = json.loads((root / "tools/release-sources.lock.json").read_text())
    if set(specifications) != {"libtool", "graphviz"}:
        raise RuntimeError("Native bottle lock must cover exactly libtool and Graphviz")
    for name, specification in specifications.items():
        if specification["version"] != sources[name]["version"]:
            raise RuntimeError(f"Native bottle/source version mismatch: {name}")


def main():
    argparse.ArgumentParser(description=__doc__).parse_args()
    require_ci_runner()
    specifications = json.loads((ROOT / "tools/ci/native-bottles.lock.json").read_text())
    validate_locks(ROOT, specifications)
    fetch_sources(ROOT)
    bottles = fetch_bottles(ROOT, specifications)
    install_bottles(specifications, bottles)
    print("Verified source archives and installed pinned native release dependencies")


if __name__ == "__main__":
    main()
