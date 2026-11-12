"""Validate main-based releases and publish complete assets through a draft."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]
ASSET_NAMES = ("CodeCallNavigator.zip", "CodeCallNavigatorEngine.zip", "CodeCallNavigator-source.tar.gz")


def git(root, *arguments):
    return subprocess.check_output(["git", "-C", str(root), *arguments], text=True).strip()


def event_commit(root, ref):
    if not ref.startswith("refs/tags/") and ref != "refs/heads/main":
        raise RuntimeError("Release verification requires a tag or main ref")
    commit = git(root, "rev-parse", "--verify", "HEAD^{commit}")
    if ref.startswith("refs/tags/"):
        if git(root, "rev-parse", "--verify", ref + "^{commit}") != commit:
            raise RuntimeError("The event ref no longer matches this checkout")
    return commit


def verify_main_commit(root, ref):
    commit = event_commit(root, ref)
    result = subprocess.run(["git", "-C", str(root), "merge-base", "--is-ancestor", commit, "origin/main"])
    if result.returncode:
        raise RuntimeError("The selected commit is not in main history")
    return commit


def read_checksums(output):
    lines = (output / "SHA256SUMS").read_text().splitlines()
    if len(lines) != len(ASSET_NAMES):
        raise RuntimeError("Release checksums must contain one entry per archive")
    checksums = dict(line.split()[::-1] for line in lines)
    if set(checksums) != set(ASSET_NAMES):
        raise RuntimeError("Release checksums must cover exactly the three release archives")
    return checksums


def verify_assets(output):
    for name, checksum in read_checksums(output).items():
        verify_file_checksum(output / name, checksum)


def verify_file_checksum(path, checksum):
    with path.open("rb") as stream:
        actual = hashlib.file_digest(stream, "sha256").hexdigest()
    if actual != checksum:
        raise RuntimeError(f"Release checksum mismatch: {path.name}")


def gh(*arguments, allow_missing=False):
    result = subprocess.run(["gh", *map(str, arguments)], capture_output=True, text=True)
    if result.returncode == 0:
        return result.stdout
    missing = "release not found" in result.stderr.lower() or "HTTP 404" in result.stderr
    if allow_missing and missing:
        return None
    raise RuntimeError(result.stderr.strip())


def release_notes(tag, repository, commit):
    source = f"https://github.com/{repository}/releases/download/{quote(tag, safe='')}/{ASSET_NAMES[2]}"
    return (f"<!-- CodeCallNavigator release {commit} -->\n\n"
            "Apple Silicon macOS build (macOS 27 or newer). Ad hoc signed, not notarized.\n\n"
            "Download CodeCallNavigator.zip and extract the app. The engine ZIP is optional.\n\n"
            f"[Matching complete source archive]({source}) includes pinned upstream sources, "
            "patches, Cargo dependency sources and rebuild instructions. GitHub's automatic "
            "source archives do not include all corresponding dependency sources.\n\n"
            "Verify the downloads against SHA256SUMS. AGPLv3; third-party terms are preserved. "
            "No guaranteed support, response or fixes.\n")


def prepare_draft(tag, repository, commit, notes_path):
    existing = gh("release", "view", tag, "--repo", repository, "--json", "isDraft,body", allow_missing=True)
    if existing is None:
        gh("release", "create", tag, "--repo", repository, "--verify-tag", "--target", commit,
           "--title", tag, "--draft", "--notes-file", notes_path)
        return True
    release = json.loads(existing)
    if not release["isDraft"]:
        print(f"Release {tag} is already published; leaving its assets unchanged")
        return False
    if f"<!-- CodeCallNavigator release {commit} -->" not in release["body"]:
        raise RuntimeError("Refusing to overwrite an unrelated release draft")
    return True


def publish(output, tag, repository, commit):
    verify_assets(output)
    with tempfile.TemporaryDirectory(prefix="codecallnavigator-release-notes-") as temporary:
        notes = Path(temporary) / "notes.md"
        notes.write_text(release_notes(tag, repository, commit))
        if not prepare_draft(tag, repository, commit, notes):
            return
        assets = [output / name for name in (*ASSET_NAMES, "SHA256SUMS")]
        gh("release", "upload", tag, "--repo", repository, "--clobber", *assets)
        gh("release", "edit", tag, "--repo", repository, "--draft=false")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("validate", "verify-assets", "publish"))
    args = parser.parse_args()
    output = ROOT / "dist"
    if args.operation == "verify-assets":
        verify_assets(output)
        return
    ref = os.environ["GITHUB_REF"]
    commit = verify_main_commit(ROOT, ref)
    if args.operation == "publish":
        if not ref.startswith("refs/tags/"):
            raise RuntimeError("Only tag events can publish a release")
        publish(output, ref.removeprefix("refs/tags/"), os.environ["GITHUB_REPOSITORY"], commit)
    print(f"Verified main commit: {commit}")


if __name__ == "__main__":
    main()
