"""Pinned dependency patches preserve exact migration and reject unrelated edits."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import bootstrap


ROOT = Path(__file__).resolve().parents[1]


class PatchMigrationCases:
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix=f"codecallnavigator-{self.dependency}-patch-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.repository = self.root / "vendor" / self.dependency
        self.repository.mkdir(parents=True)
        self.spec = json.loads((ROOT / "analyzers.lock.json").read_text())[self.dependency]
        self.original_root = bootstrap.ROOT
        self.addCleanup(setattr, bootstrap, "ROOT", self.original_root)
        bootstrap.ROOT = self.root
        self.prepare_repository()
        self.copy_patches()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.repository), *arguments], stderr=subprocess.STDOUT)

    def checkout_diff(self):
        return bootstrap.checkout_patch(["git", "-C", str(self.repository)])

    def prepare_repository(self):
        patch = (ROOT / self.spec["patch"]).read_text()
        paths = [line.removeprefix("--- a/") for line in patch.splitlines() if line.startswith("--- a/")]
        for relative in paths:
            contents = subprocess.check_output(["git", "-C", str(ROOT / "vendor" / self.dependency), "show", "HEAD:" + relative])
            destination = self.repository / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(contents)
        self.git("init", "--quiet")
        # Upstream blobs may contain mixed line endings; preserve their exact bytes.
        self.git("config", "core.autocrlf", "false")
        self.git("config", "core.abbrev", str(self.abbreviation))
        self.git("add", ".")
        self.git("-c", "commit.gpgsign=false", "-c", "user.name=Patch test", "-c", "user.email=patch-test@example.invalid", "commit", "--quiet", "-m", "Pinned test sources")

    def copy_patches(self):
        for key in ["patch", *sorted(key for key in self.spec if key.endswith("_patch"))]:
            destination = self.root / self.spec[key]
            destination.parent.mkdir(exist_ok=True)
            shutil.copy2(ROOT / self.spec[key], destination)

    def test_clean_checkout_and_prior_patch_reach_the_exact_pinned_diff(self):
        expected = (self.root / self.spec["patch"]).read_bytes()
        bootstrap.apply_patch(self.spec, self.dependency)
        self.assertEqual(self.checkout_diff(), expected)
        for key in sorted(key for key in self.spec if key.endswith("_patch")):
            with self.subTest(checkpoint=key):
                self.git("apply", "--reverse", str(self.root / self.spec["patch"]))
                self.git("apply", str(self.root / self.spec[key]))
                bootstrap.apply_patch(self.spec, self.dependency)
                self.assertEqual(self.checkout_diff(), expected)
        bootstrap.apply_patch(self.spec, self.dependency)
        self.assertEqual(self.checkout_diff(), expected)

    def test_unknown_upstream_edits_are_rejected_without_modification(self):
        self.git("apply", str(self.root / self.spec["previous_patch"]))
        source = self.repository / self.source
        source.write_text(source.read_text() + "\n// unrelated local change\n")
        original = self.checkout_diff()
        with self.assertRaisesRegex(RuntimeError, "Dependency source differs"):
            bootstrap.apply_patch(self.spec, self.dependency)
        self.assertEqual(self.checkout_diff(), original)


class RunePatchMigration(PatchMigrationCases, unittest.TestCase):
    dependency = "rune"
    source = "crates/rune/src/shader.wgsl"
    abbreviation = 8


class CallAtlasPatchMigration(PatchMigrationCases, unittest.TestCase):
    dependency = "callatlas"
    source = "src/CallAtlas.Core/CallGraphBuilder.cs"
    abbreviation = 7

    def test_pre_flow_checkpoint_preserves_the_recorded_exporter_baseline(self):
        baseline = json.loads((ROOT / "fixtures/flow/csharp-baselines/pre-export-baseline.json").read_text())
        checkpoint = ROOT / self.spec["pre_flow_patch"]
        self.assertEqual(hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                         baseline["patch_sha256"])

    def test_pre_source_encoding_checkpoint_preserves_the_flow_exporter(self):
        checkpoint = ROOT / self.spec["pre_source_encoding_patch"]
        self.assertEqual(hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                         "173f6ea6a771f5d2dded30f68319c3b7aa8dd4384a61fa43ad2f0bd71b5f3a7f")

    def test_pre_cleanup_checkpoint_preserves_the_ingestion_baseline(self):
        baseline = json.loads((ROOT / "fixtures/flow/csharp-baselines/ingestion-checkpoint.json").read_text())
        checkpoint = ROOT / self.spec["pre_cleanup_patch"]
        self.assertEqual(hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                         baseline["patch_sha256"])

    def test_pre_execution_checkpoint_preserves_the_cleanup_baseline(self):
        checkpoint = ROOT / self.spec["pre_execution_patch"]
        self.assertEqual(hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                         "c36315a4e60f2a0a547483bb4c253add2ed3dad9b9581329384004400bbaf572")


if __name__ == "__main__":
    unittest.main()
