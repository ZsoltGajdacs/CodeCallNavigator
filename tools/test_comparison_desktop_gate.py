"""The opt-in Git gate cannot intercept arbitrary projects or Git commands."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from comparison_desktop_gate import Gate, resolution_arguments


class ComparisonGitGateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.project = self.root / "project"
        self.project.mkdir()
        self.output = self.root / "output"
        self.output.mkdir()
        self.commit = "a" * 40
        self.gate = Gate(self.project, self.output, "b" * 32, self.commit)

    def test_only_exact_project_base_ref_and_argument_vector_can_enter(self):
        self.gate.install()
        self.gate.arm()
        arguments = resolution_arguments(self.project, self.commit)
        for changed in [arguments[1:], [*arguments, "extra"],
                        resolution_arguments(self.root, self.commit),
                        [*arguments[:-1], "refs/tags/fixture-after^{commit}"]]:
            subprocess.run([self.output / "path/git", *changed],
                           capture_output=True, timeout=5)
            self.assertFalse((self.output / "gate-held.json").exists())

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

    def test_noncanonical_projects_and_unknown_tokens_are_rejected(self):
        alias = self.root / "alias"
        alias.symlink_to(self.project)
        with self.assertRaises(ValueError):
            Gate(alias, self.output, "b" * 32, self.commit)
        with self.assertRaises(ValueError):
            Gate(self.project, self.output, "arbitrary", self.commit)


if __name__ == "__main__":
    unittest.main()
