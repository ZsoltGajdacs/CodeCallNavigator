"""Source releases preserve rebuild inputs and reject incomplete provenance."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import release_sources


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], stderr=subprocess.DEVNULL)


class ReleaseSourceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        git(self.root, "init")
        git(self.root, "config", "user.name", "Release test")
        git(self.root, "config", "user.email", "release@example.invalid")
        (self.root / "source.rs").write_text("original source\n")
        git(self.root, "add", ".")
        git(self.root, "commit", "-m", "Source")

    def test_dirty_or_hidden_source_cannot_be_released(self):
        git(self.root, "update-index", "--assume-unchanged", "source.rs")
        (self.root / "source.rs").write_text("uncommitted source\n")
        with self.assertRaisesRegex(RuntimeError, "Commit"):
            release_sources.require_clean_source(self.root)

    def test_archive_preserves_build_scripts_without_private_or_build_files(self):
        (self.root / ".gitignore").write_text("private/\nbuild/\n")
        (self.root / "build.sh").write_text("#!/bin/sh\necho build\n")
        git(self.root, "add", ".")
        git(self.root, "commit", "-m", "Build inputs")
        (self.root / "private").mkdir()
        (self.root / "private/token").write_text("private token")
        archive = self.root / "private/source.tar"
        release_sources.archive_project(self.root, archive)
        with tarfile.open(archive) as source:
            names = source.getnames()
            self.assertIn("build.sh", names)
            self.assertIn("source.rs", names)
            self.assertFalse(any("private" in name or ".git/" in name for name in names))

    def test_upstream_bundle_restores_exact_revision_without_network(self):
        destination = self.root / "upstream.bundle"
        release_sources.bundle_repository(self.root, destination)
        restored = self.root / "restored"
        subprocess.run(["git", "clone", str(destination), str(restored)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.assertEqual(git(restored, "rev-parse", "HEAD"), git(self.root, "rev-parse", "HEAD"))
        self.assertEqual((restored / "source.rs").read_text(), "original source\n")

    def test_missing_or_corrupt_dependency_source_is_rejected(self):
        spec = {"filename": "dependency.tar.gz", "sha256": hashlib.sha256(b"source").hexdigest()}
        with self.assertRaisesRegex(RuntimeError, "Missing"):
            release_sources.verified_archive(self.root, spec)
        (self.root / spec["filename"]).write_bytes(b"wrong source")
        with self.assertRaisesRegex(RuntimeError, "checksum"):
            release_sources.verified_archive(self.root, spec)
        (self.root / spec["filename"]).write_bytes(b"source")
        self.assertEqual(release_sources.verified_archive(self.root, spec).read_bytes(), b"source")

    def test_changed_binary_or_mismatched_source_is_rejected(self):
        (self.root / ".gitignore").write_text("dist/\n")
        git(self.root, "add", ".")
        git(self.root, "commit", "-m", "Ignore outputs")
        resources = self.root / "dist/Resources"
        resources.mkdir(parents=True)
        executable = resources.parent / "app"
        executable.write_bytes(b"built executable")
        release_sources.write_build_manifest(self.root, resources)
        release_sources.verify_build_manifest(self.root, resources)
        executable.write_bytes(b"different executable")
        with self.assertRaisesRegex(RuntimeError, "changed"):
            release_sources.verify_build_manifest(self.root, resources)

    def test_source_release_restores_project_and_preserves_checked_sources(self):
        (self.root / ".gitignore").write_text("dist/\nvendor/\n")
        (self.root / "tools").mkdir()
        (self.root / "docs").mkdir()
        (self.root / "docs/public-releases.md").write_text("Rebuild instructions\n")
        content = b"complete dependency source and build scripts"
        specification = {"filename": "aviator-1.tar.gz", "version": "1",
                         "sha256": hashlib.sha256(content).hexdigest()}
        (self.root / "tools/release-sources.lock.json").write_text(json.dumps({"aviator": specification}))
        git(self.root, "add", ".")
        git(self.root, "commit", "-m", "Release inputs")
        for name in ("callatlas", "rune", "java-callgraph2"):
            path = self.root / "vendor" / name
            path.parent.mkdir(exist_ok=True)
            subprocess.run(["git", "clone", str(self.root), str(path)], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        cache = self.root / "vendor/release-sources"
        cache.mkdir()
        (cache / specification["filename"]).write_bytes(content)
        resources = self.root / "dist/Engine/Resources"
        jars = resources / "analyzers/java-bytecode/lib"
        jars.mkdir(parents=True)
        (jars / "aviator-1.jar").write_bytes(b"binary dependency")
        release_sources.write_build_manifest(self.root, resources)
        real_run = subprocess.run

        def vendor_or_run(arguments, **kwargs):
            if arguments[:2] == ["cargo", "vendor"]:
                destination = Path(arguments[-1])
                destination.mkdir()
                (destination / "dependency.rs").write_text("dependency source\n")
                return subprocess.CompletedProcess(arguments, 0, stdout=f'directory = "{destination}"\n')
            return real_run(arguments, **kwargs)

        with patch.object(release_sources.subprocess, "run", side_effect=vendor_or_run):
            archive = release_sources.prepare_source_release(self.root, [resources], self.root / "dist")
        extracted = self.root / "dist/extracted"
        with tarfile.open(archive) as source:
            source.extractall(extracted, filter="data")
        preserved = extracted / "CodeCallNavigator-source"
        manifest = json.loads((preserved / "SOURCE_MANIFEST.json").read_text())
        for name, checksum in manifest["files"].items():
            self.assertEqual(release_sources.digest(preserved / name), checksum)
        self.assertEqual((preserved / "cargo-config.toml").read_text(), 'directory = "../cargo-vendor"\n')
        self.assertEqual((preserved / "dependency-sources/aviator-1.tar.gz").read_bytes(), content)
        restored = self.root / "dist/restored"
        subprocess.run(["git", "clone", str(preserved / "project.bundle"), str(restored)], check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.assertEqual(git(restored, "rev-parse", "HEAD"), git(self.root, "rev-parse", "HEAD"))


if __name__ == "__main__":
    unittest.main()
