"""Pinned Rune patch installation must preserve exact migration and reject unrelated edits."""
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import bootstrap


ROOT = Path(__file__).resolve().parents[1]


class RunePatchMigration(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codecallnavigator-rune-patch-")
        self.root = Path(self.temporary.name)
        self.repository = self.root / "vendor/rune"
        self.repository.mkdir(parents=True)
        self.spec = json.loads((ROOT / "analyzers.lock.json").read_text())["rune"]
        self.original_root = bootstrap.ROOT
        bootstrap.ROOT = self.root
        self.prepare_repository()
        self.copy_patches()

    def tearDown(self):
        bootstrap.ROOT = self.original_root
        self.temporary.cleanup()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.repository), *arguments], stderr=subprocess.STDOUT)

    def checkout_diff(self):
        return bootstrap.checkout_patch(["git", "-C", str(self.repository)])

    def prepare_repository(self):
        patch = (ROOT / self.spec["patch"]).read_text()
        paths = [line.removeprefix("--- a/") for line in patch.splitlines() if line.startswith("--- a/")]
        for relative in paths:
            contents = subprocess.check_output(["git", "-C", str(ROOT / "vendor/rune"), "show", "HEAD:" + relative])
            destination = self.repository / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(contents)
        self.git("init", "--quiet")
        self.git("config", "core.abbrev", "8")
        self.git("add", ".")
        self.git("-c", "commit.gpgsign=false", "-c", "user.name=Patch test", "-c", "user.email=patch-test@example.invalid", "commit", "--quiet", "-m", "Pinned test sources")

    def copy_patches(self):
        for key in ["patch", *sorted(key for key in self.spec if key.endswith("_patch"))]:
            destination = self.root / self.spec[key]
            destination.parent.mkdir(exist_ok=True)
            shutil.copy2(ROOT / self.spec[key], destination)

    def test_clean_checkout_and_prior_patch_reach_the_exact_pinned_diff(self):
        expected = (self.root / self.spec["patch"]).read_bytes()
        bootstrap.apply_patch(self.spec, "rune")
        self.assertEqual(self.checkout_diff(), expected)
        for key in sorted(key for key in self.spec if key.endswith("_patch")):
            with self.subTest(checkpoint=key):
                self.git("apply", "--reverse", str(self.root / self.spec["patch"]))
                self.git("apply", str(self.root / self.spec[key]))
                bootstrap.apply_patch(self.spec, "rune")
                self.assertEqual(self.checkout_diff(), expected)
        bootstrap.apply_patch(self.spec, "rune")
        self.assertEqual(self.checkout_diff(), expected)

    def test_unknown_upstream_edits_are_rejected_without_modification(self):
        self.git("apply", str(self.root / self.spec["previous_patch"]))
        shader = self.repository / "crates/rune/src/shader.wgsl"
        shader.write_text(shader.read_text() + "\n// unrelated local change\n")
        original = self.checkout_diff()
        with self.assertRaisesRegex(RuntimeError, "Dependency source differs"):
            bootstrap.apply_patch(self.spec, "rune")
        self.assertEqual(self.checkout_diff(), original)


if __name__ == "__main__":
    unittest.main()
