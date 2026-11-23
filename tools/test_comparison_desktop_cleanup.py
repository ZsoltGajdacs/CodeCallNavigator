"""A real test-owned parent and gated Git group must clean up before the parent is stopped."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch, Mock

from comparison_desktop_cleanup import OwnedRun, Process, group_alive, wait_groups, query_leases, stop_verified, termination_snapshot, wait_terminated
from comparison_desktop_gate import Gate, resolution_arguments


class ComparisonFailureCleanupTests(unittest.TestCase):
    def test_failure_disarms_the_owned_gate_and_reaps_the_query_before_parent_cleanup(self):
        self.run_owned_failure(reap_child=True)

    def test_unreaping_parent_is_closed_before_zombie_group_absence_is_required(self):
        audit = self.run_owned_failure(reap_child=False)
        self.assertEqual(audit["pre_parent"]["status"], "awaiting_parent_reap")
        self.assertTrue(any(row["state"].startswith("Z") for snapshot in audit["pre_parent"]["snapshots"] for row in snapshot["members"]))
        self.assertEqual(audit["post_parent"]["status"], "cleared")
        self.assertEqual(audit["post_parent"]["members"], [])

    def run_owned_failure(self, reap_child):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            project, output = root / "project", root / "output"
            project.mkdir()
            output.mkdir()
            engine = root / "test-engine"
            base_commit = "a" * 40
            gate = Gate(project, output, "b" * 32, base_commit, engine)
            gate.install()
            gate.arm()
            arguments = [str(output / "path/git"), *resolution_arguments(project, base_commit)]
            engine.write_text("#!/usr/bin/python3\nimport subprocess,sys,time,pathlib\n"
                              f"subprocess.call({arguments!r})\n"
                              f"while not pathlib.Path({str(root / 'allow-session-exit')!r}).exists():time.sleep(0.01)\n")
            engine.chmod(0o755)
            parent_tail = "child.wait()" if reap_child else "import time;time.sleep(30)"
            parent_code = ("import subprocess; "
                           f"child=subprocess.Popen({[str(engine), 'query-session']!r},start_new_session=True); "
                           + parent_tail)
            parent = subprocess.Popen([sys.executable, "-c", parent_code],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            leases = []
            try:
                gate.wait(parent, 5)
                leases = query_leases(parent, engine)
                self.assertEqual(len(leases), 1)
                self.assertTrue(group_alive(leases[0].group))
                audit = OwnedRun(parent, engine, gate, output).close()
                self.assertFalse(group_alive(leases[0].group),
                                 "Stopping only the app leaves the owned gated query group alive")
                self.assertFalse((output / "gate-armed.json").exists())
            finally:
                gate.disarm()
                (root / "allow-session-exit").touch()
                if parent.poll() is None:
                    parent.terminate()
                parent.wait(timeout=5)
                self.assertTrue(wait_groups(leases, 5), "Test-owned query group did not leave after disarm")

            return audit

    def test_stopped_processes_are_not_quiescent_and_state_changes_do_not_change_identity(self):
        lease = Process(123, 100, 123, "same-start", "/owned/engine query-session", state="R")
        sleeping = Process(123, 100, 123, "same-start", "/owned/engine query-session", state="S")
        self.assertEqual(lease, sleeping)
        with patch("comparison_desktop_cleanup.processes", return_value=[sleeping]), \
             patch("comparison_desktop_cleanup.group_alive", return_value=True), \
             patch("comparison_desktop_cleanup.os.killpg") as send:
            stop_verified(lease, 15)
            send.assert_called_once_with(123, 15)
        member = {"pid": 123, "ppid": 100, "pgid": 123, "state": "T"}
        with patch("comparison_desktop_cleanup.group_members", return_value=[member]):
            audit = {"snapshots": []}
            self.assertEqual(wait_terminated([lease], 0, audit)["status"], "running")

    def test_unknown_state_retains_leases_and_snapshots_without_signaling(self):
        with tempfile.TemporaryDirectory() as temporary:
            lease = Process(123, 100, 123, "same-start", "/owned/engine query-session", state="S")
            member = {"pid": 123, "ppid": 100, "pgid": 123, "state": "?"}
            app, gate = Mock(), Mock(project=Path(temporary), observed=None)
            app.poll.return_value = None
            with patch("comparison_desktop_cleanup.query_leases", return_value=[lease]), \
                 patch("comparison_desktop_cleanup.group_members", return_value=[member]), \
                 patch("comparison_desktop_cleanup.wait_groups", return_value=False), \
                 patch("comparison_desktop_cleanup.os.killpg") as send, \
                 patch("csharp_desktop_acceptance.cleanup_native") as close_parent:
                with self.assertRaisesRegex(RuntimeError, "preserve fixture"):
                    OwnedRun(app, Path("/owned/engine"), gate, Path(temporary)).close()
                send.assert_not_called()
                close_parent.assert_called_once()
            audit = json.loads((Path(temporary) / "query-cleanup.json").read_text())
            self.assertEqual(audit["status"], "inconclusive")
            self.assertEqual(audit["pre_parent"]["leases"][0]["pid"], lease.pid)
            self.assertEqual(audit["pre_parent"]["snapshots"][0]["status"], "unknown")
            self.assertEqual(audit["pre_parent"]["snapshots"][0]["members"], [member])

    def test_numeric_group_membership_needs_no_signal_zero_and_keeps_zombie_members(self):
        from types import SimpleNamespace
        with patch("comparison_desktop_cleanup.os.killpg", side_effect=PermissionError("signal status denied")) as probe, \
             patch("comparison_desktop_cleanup.subprocess.run", return_value=SimpleNamespace(stdout="123\n456\n")) as snapshot:
            self.assertTrue(group_alive(123))
            self.assertFalse(group_alive(789))
            probe.assert_not_called()
            self.assertEqual(snapshot.call_args.args[0], ["/bin/ps", "-axo", "pgid="])
        with patch("comparison_desktop_cleanup.subprocess.run", side_effect=PermissionError("inspection denied")):
            with self.assertRaises(PermissionError):
                group_alive(123)
        with patch("comparison_desktop_cleanup.subprocess.run", return_value=SimpleNamespace(stdout="unknown\n")):
            with self.assertRaises(ValueError):
                group_alive(123)

    def test_reused_or_mismatched_group_identity_never_receives_a_signal(self):
        lease = Process(123, 100, 123, "original", "/owned/engine query-session")
        reused = Process(123, 1, 123, "different", "/unrelated/process")
        with patch("comparison_desktop_cleanup.processes", return_value=[reused]), \
             patch("comparison_desktop_cleanup.group_alive", return_value=True), \
             patch("comparison_desktop_cleanup.os.killpg") as signal:
            with self.assertRaises(ValueError):
                stop_verified(lease, 15)
            signal.assert_not_called()

    def test_inconclusive_inspection_disarms_only_owned_token_and_reports_preserved_diagnostics(self):
        with tempfile.TemporaryDirectory() as temporary:
            gate = Mock(project=Path(temporary), observed=None)
            app = Mock()
            app.poll.return_value = None
            with patch("comparison_desktop_cleanup.query_leases", side_effect=ValueError("inspection unavailable")), \
                 patch("csharp_desktop_acceptance.cleanup_native") as cleanup:
                with self.assertRaisesRegex(RuntimeError, "preserve fixture"):
                    OwnedRun(app, Path("/owned/engine"), gate, Path(temporary)).close()
                gate.disarm.assert_called_once()
                cleanup.assert_called_once()
            audit = json.loads((Path(temporary) / "query-cleanup.json").read_text())
            self.assertEqual(audit["status"], "inconclusive")
            self.assertTrue(audit["diagnostics_preserved"])


if __name__ == "__main__":
    unittest.main()
