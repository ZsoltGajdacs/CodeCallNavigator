"""Packaging must include only the reviewed source revisions and exact identity patch."""
import importlib.util
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
SPEC = importlib.util.spec_from_file_location("codecallnavigator_package", Path(__file__).resolve().parents[1] / "tools/package.py")
PACKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGE)


def git(repository, *arguments):
    return subprocess.check_output(["git", "-C", str(repository), *arguments], stderr=subprocess.DEVNULL)


class SourceProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codecallnavigator-package-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.specifications = {}
        for name in ("callatlas", "rune", "java-callgraph2"):
            repository = self.root / "vendor" / name
            repository.mkdir(parents=True)
            git(repository, "init")
            git(repository, "config", "user.email", "test@example.invalid")
            git(repository, "config", "user.name", "Package Test")
            (repository / "source.rs").write_text("reviewed source\n")
            git(repository, "add", "source.rs")
            git(repository, "commit", "-m", "Reviewed dependency")
            self.specifications[name] = {"revision": git(repository, "rev-parse", "HEAD").decode().strip()}
        self.write_lock()

    def write_lock(self):
        (self.root / "analyzers.lock.json").write_text(json.dumps(self.specifications))

    def test_staged_dependency_edit_is_rejected_without_altering_it(self):
        repository = self.root / "vendor/rune"
        (repository / "source.rs").write_text("unreviewed staged source\n")
        git(repository, "add", "source.rs")
        with patch.object(PACKAGE, "ROOT", self.root):
            with self.assertRaisesRegex(RuntimeError, "Unvalidated local changes"):
                PACKAGE.verify_dependency_sources()
        self.assertEqual((repository / "source.rs").read_text(), "unreviewed staged source\n")
        self.assertTrue(git(repository, "diff", "--cached"))

    def test_index_flags_cannot_hide_dependency_edits(self):
        repository = self.root / "vendor/java-callgraph2"
        for flag in ("--assume-unchanged", "--skip-worktree"):
            git(repository, "update-index", flag, "source.rs")
            (repository / "source.rs").write_text("unrecorded upstream change\n")
            before = (repository / ".git/index").read_bytes()
            with patch.object(PACKAGE, "ROOT", self.root):
                with self.assertRaisesRegex(RuntimeError, "Unvalidated local changes"):
                    PACKAGE.verify_dependency_sources()
            self.assertEqual((repository / ".git/index").read_bytes(), before)
            git(repository, "update-index", flag.replace("--", "--no-", 1), "source.rs")
            (repository / "source.rs").write_text("reviewed source\n")

    def test_external_diff_cannot_hide_dependency_edits(self):
        repository = self.root / "vendor/java-callgraph2"
        (repository / "source.rs").write_text("unrecorded upstream change\n")
        git(repository, "config", "diff.external", "/usr/bin/true")
        with patch.object(PACKAGE, "ROOT", self.root):
            with self.assertRaisesRegex(RuntimeError, "Unvalidated local changes"):
                PACKAGE.verify_dependency_sources()

    def test_matching_but_unlocked_patch_is_rejected(self):
        repository = self.root / "vendor/callatlas"
        (repository / "source.rs").write_text("different source\n")
        patch_file = self.root / "identity.patch"
        patch_file.write_bytes(git(repository, "diff", "HEAD", "--binary"))
        self.specifications["callatlas"].update({"patch": "identity.patch", "patch_sha256": "0" * 64})
        self.write_lock()
        with patch.object(PACKAGE, "ROOT", self.root):
            with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
                PACKAGE.verify_dependency_sources()

    def test_verified_patch_added_source_is_included_and_extra_source_rejected(self):
        repository = self.root / "vendor/callatlas"
        (repository / "New adapter.cs").write_text("reviewed added source\n")
        git(repository, "add", ".")
        expected = git(repository, "diff", "--cached", "HEAD", "--binary")
        git(repository, "reset", "--quiet", "HEAD")
        (self.root / "identity.patch").write_bytes(expected)
        self.specifications["callatlas"].update({"patch": "identity.patch", "patch_sha256": hashlib.sha256(expected).hexdigest()})
        self.write_lock()
        index = repository / ".git/index"
        before = index.read_bytes()
        with patch.object(PACKAGE, "ROOT", self.root):
            PACKAGE.verify_dependency_sources()
            (repository / "Unrelated.cs").write_text("unrecorded source\n")
            with self.assertRaisesRegex(RuntimeError, "Unvalidated local changes"):
                PACKAGE.verify_dependency_sources()
        self.assertEqual(index.read_bytes(), before)

    def test_truncated_empty_patch_cannot_ship_unpatched_engine(self):
        (self.root / "identity.patch").write_bytes(b"")
        self.specifications["callatlas"].update({"patch": "identity.patch", "patch_sha256": "0" * 64})
        self.write_lock()
        with patch.object(PACKAGE, "ROOT", self.root):
            with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
                PACKAGE.verify_dependency_sources()


class JavaAdapterCapabilityTests(unittest.TestCase):
    def test_installed_adapter_has_matching_correlation_capability(self):
        distribution = PACKAGE.ROOT / "tools/java-bytecode/build/install/codecallnavigator-bytecode"
        if not (distribution / "lib/codecallnavigator-bytecode.jar").is_file():
            self.skipTest("Build the Java adapter before testing packaged resources")
        PACKAGE.verify_java_adapter(distribution)

    def test_incomplete_adapter_is_rejected_before_packaging(self):
        with tempfile.TemporaryDirectory() as temporary:
            distribution = Path(temporary)
            (distribution / "lib").mkdir()
            jar = distribution / "lib/codecallnavigator-bytecode.jar"
            with zipfile.ZipFile(jar, "w") as archive:
                archive.writestr("codecallnavigator/bytecode/SourceInventory.class", b"kind_v8")
            with self.assertRaisesRegex(RuntimeError, "lacks kind_v8"):
                PACKAGE.verify_java_adapter(distribution)

    def test_complete_adapter_with_legacy_inventory_marker_is_rejected(self):
        installed = PACKAGE.ROOT / "tools/java-bytecode/build/install/codecallnavigator-bytecode/lib/codecallnavigator-bytecode.jar"
        if not installed.is_file():
            self.skipTest("Build the Java adapter before testing packaged resources")
        with tempfile.TemporaryDirectory() as temporary:
            distribution = Path(temporary)
            (distribution / "lib").mkdir()
            with zipfile.ZipFile(installed) as original, zipfile.ZipFile(distribution / "lib/codecallnavigator-bytecode.jar", "w") as legacy:
                for name in original.namelist():
                    contents = original.read(name)
                    if name == "codecallnavigator/bytecode/SourceInventory.class":
                        self.assertIn(b"kind_v8", contents)
                        contents = contents.replace(b"kind_v8", b"kind_v7")
                    legacy.writestr(name, contents)
            with self.assertRaisesRegex(RuntimeError, "lacks kind_v8"):
                PACKAGE.verify_java_adapter(distribution)

    def test_invalid_adapter_archive_has_actionable_error(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(RuntimeError, "rebuild matching analyzer resources"):
                PACKAGE.verify_java_adapter(Path(temporary))


if __name__ == "__main__":
    unittest.main()
