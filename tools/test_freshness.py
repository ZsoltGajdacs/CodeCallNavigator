"""Narrow tests for freshness measurement invariants."""
import argparse
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from freshness_scenarios import Project
from freshness_measurement import distribution, summarize


class MeasurementTests(unittest.TestCase):
    def test_p95_is_nearest_rank_and_empty_is_unavailable(self):
        self.assertEqual(distribution([]), None)
        self.assertEqual(distribution(list(range(1, 21)))["p95"], 19)

    def test_failed_measurements_do_not_claim_equivalence(self):
        result = summarize([{"language": "maven", "scenario": "body", "phase": "cold", "failure": "missing JDK"}])
        self.assertIsNone(result["maven/body"]["equivalent"])
        self.assertEqual(result["maven/body"]["warm_records"], 0)

    def test_java_discovery_matches_directory_identity_and_build_kind(self):
        from measure_freshness import selected_targets
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = Project(root, "maven", None, Path(__file__).resolve().parents[1])
            maven = {"kind": "maven", "language": "java", "path": str(root), "label": "probe"}
            gradle = dict(maven, kind="gradle")
            self.assertEqual(selected_targets(project, {"targets": [gradle, maven]}), [maven])
            with self.assertRaises(RuntimeError):
                selected_targets(project, {"targets": [gradle]})

    def test_timeout_cleans_worker_in_separate_session_after_root_exit(self):
        from freshness_measurement import run
        try:
            subprocess.run(["/bin/ps", "-p", str(os.getpid())], check=True, capture_output=True)
        except (OSError, subprocess.SubprocessError):
            self.skipTest("process inventory unavailable in this sandbox")
        child = "import signal,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(60)"
        parent = "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c'," + repr(child) + "],start_new_session=True); print(p.pid,flush=True);time.sleep(60)"
        result = run([sys.executable, "-c", parent], timeout=.5)
        self.assertTrue(result["timed_out"])
        self.assertEqual(result["cleanup_errors"], [])
        pid = int(result["stdout"].strip())
        status = subprocess.run(["/bin/ps", "-p", str(pid), "-o", "stat="], capture_output=True, text=True).stdout.strip()
        self.assertTrue(not status or status.startswith("Z"), status)

    def test_rename_and_deletion_change_source_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Project(Path(directory), "maven", None, Path(__file__).resolve().parents[1])
            project.reset("rename")
            project.edit("rename", 1)
            self.assertFalse(project.extra.exists())
            self.assertTrue(project.renamed.exists())
            project.reset("deletion")
            project.edit("deletion", 1)
            self.assertFalse(project.extra.exists())
            self.assertFalse(project.renamed.exists())

    def test_edits_are_saved_and_reset_removes_added_membership(self):
        with tempfile.TemporaryDirectory() as directory:
            project = Project(Path(directory), "maven", None, Path(__file__).resolve().parents[1])
            project.reset()
            before = project.source.read_bytes()
            project.edit("body", 1)
            self.assertNotEqual(project.source.read_bytes(), before)
            project.edit("addition", 1)
            self.assertTrue(project.extra.exists())
            project.reset()
            self.assertEqual(project.source.read_bytes(), before)
            self.assertFalse(project.extra.exists())


@unittest.skipUnless(os.environ.get("CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE") and os.environ.get("CODECALLNAVIGATOR_TS_NODE_MODULES"), "set real CLI and TypeScript modules for compiler integration")
class CompilerJourneyTests(unittest.TestCase):
    def test_saved_edit_refresh_and_failure_recovery(self):
        from measure_freshness import execute
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "observations"
            arguments = argparse.Namespace(executable=Path(os.environ["CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE"]), output=output,
                typescript_modules=Path(os.environ["CODECALLNAVIGATOR_TS_NODE_MODULES"]), languages=["typescript"], scenarios=["body"], runs=1, timeout=120)
            self.assertEqual(execute(arguments), 0)
            summary = json.loads((output / "summary.json").read_text())
            self.assertTrue(summary["summary"]["typescript/body"]["equivalent"])
            self.assertEqual(summary["summary"]["typescript/body"]["warm_records"], 1)
            self.assertTrue(summary["failure_journeys"]["typescript"]["previous_artifact_preserved"])
            self.assertTrue(summary["failure_journeys"]["typescript"]["generation_mismatch_rejected"])


if __name__ == "__main__":
    unittest.main()
