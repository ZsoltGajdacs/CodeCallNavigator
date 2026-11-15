"""The opt-in Git gate cannot intercept arbitrary projects or Git commands."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from comparison_desktop_gate import Gate, matches


class ComparisonGitGateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.project = self.root / "project"
        self.project.mkdir()
        self.output = self.root / "output"
        self.output.mkdir()
        self.blob = "a" * 40 + ":.codecallnavigator/graph.json"
        self.gate = Gate(self.project, self.output, self.blob, "b" * 32)

    def test_only_exact_project_blob_and_argument_vector_can_enter(self):
        arguments = ["--no-pager", "-C", str(self.project), "cat-file", "blob", self.blob]
        self.assertTrue(matches(arguments, self.project, self.blob))
        for changed in [arguments[1:], [*arguments, "extra"],
                        [*arguments[:2], str(self.root), *arguments[3:]],
                        [*arguments[:-1], "other:" + self.blob.split(":")[1]]]:
            self.assertFalse(matches(changed, self.project, self.blob))

    def test_nonmatching_real_git_delegates_without_waiting_or_producing_proof(self):
        self.gate.install()
        self.gate.arm()
        output = subprocess.run([self.output / "path/git", "--version"],
                                capture_output=True, text=True, timeout=5, check=True)
        self.assertTrue(output.stdout.startswith("git version"))
        self.assertFalse((self.output / "gate-held.json").exists())

    def test_gate_stale_state_and_unentered_release_fail_closed(self):
        self.gate.arm()
        with self.assertRaises(ValueError):
            self.gate.arm()
        with self.assertRaises(ValueError):
            self.gate.release()

    def test_outside_artifacts_noncanonical_projects_and_unknown_tokens_are_rejected(self):
        for blob in ["HEAD:.codecallnavigator/graph.json", "a" * 40 + ":other.json"]:
            with self.assertRaises(ValueError):
                Gate(self.project, self.output, blob, "b" * 32)
        alias = self.root / "alias"
        alias.symlink_to(self.project)
        with self.assertRaises(ValueError):
            Gate(alias, self.output, self.blob, "b" * 32)
        with self.assertRaises(ValueError):
            Gate(self.project, self.output, self.blob, "arbitrary")


if __name__ == "__main__":
    unittest.main()
