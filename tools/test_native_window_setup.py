"""Ownership checks must reject another application's window before any mutation."""
import unittest
import json
import plistlib
import subprocess
from types import SimpleNamespace
from unittest.mock import patch

from native_window_setup import WindowIdentity, WindowLease, select_owned
from verification.desktop_session import DesktopStatus, desktop_status, parse_console_sessions


def reply(rows=None):
    return SimpleNamespace(returncode=0, stdout=json.dumps(rows or []), stderr="")


class DesktopSessionTests(unittest.TestCase):
    def registry(self, sessions):
        return plistlib.dumps([{"IORegistryEntryChildren": [{"IOConsoleUsers": sessions}]}])

    def test_locked_console(self):
        session = {"kCGSSessionOnConsoleKey": True, "kCGSessionLoginDoneKey": True,
                   "CGSSessionScreenIsLocked": True}
        self.assertEqual(parse_console_sessions(self.registry([session])), DesktopStatus.LOCKED)

    def test_unlocked_console(self):
        for lock in ({}, {"CGSSessionScreenIsLocked": False}):
            session = {"kCGSSessionOnConsoleKey": True, "kCGSessionLoginDoneKey": True} | lock
            self.assertEqual(parse_console_sessions(self.registry([session])), DesktopStatus.READY)

    def test_missing_console(self):
        inactive = {"kCGSSessionOnConsoleKey": False, "kCGSessionLoginDoneKey": True}
        login_window = {"kCGSSessionOnConsoleKey": True, "kCGSessionLoginDoneKey": False}
        self.assertEqual(parse_console_sessions(self.registry([inactive, login_window])),
                         DesktopStatus.UNAVAILABLE)

    def test_malformed_registry(self):
        session = {"kCGSSessionOnConsoleKey": True, "kCGSessionLoginDoneKey": True,
                   "CGSSessionScreenIsLocked": "invalid"}
        for payload in (b"invalid plist", self.registry([session]), self.registry("invalid list"),
                        b"<plist><key>bad</key></plist>",
                        b"<plist><date>invalid</date></plist>"):
            self.assertEqual(parse_console_sessions(payload), DesktopStatus.UNAVAILABLE)

    @patch("verification.desktop_session.subprocess.run", side_effect=subprocess.TimeoutExpired("ioreg", 3))
    def test_probe_timeout(self, run):
        self.assertEqual(desktop_status(), DesktopStatus.UNAVAILABLE)
        self.assertEqual(run.call_args.kwargs["timeout"], 3)


class WindowSetupTests(unittest.TestCase):
    def setUp(self):
        self.identity = WindowIdentity(process_id=123, window_id=456)

    def test_foreign_pid_for_matching_window_is_rejected(self):
        with self.assertRaises(ValueError):
            select_owned([{"window-id": 456, "app-pid": 999, "window-layout": "h_tiles"}], self.identity)

    def test_duplicate_window_identity_is_rejected(self):
        row = {"window-id": 456, "app-pid": 123, "window-layout": "h_tiles"}
        with self.assertRaises(ValueError):
            select_owned([row, row], self.identity)

    def test_missing_window_is_not_replaced_by_another_window(self):
        self.assertIsNone(select_owned([{"window-id": 457, "app-pid": 123}], self.identity))

    def test_unknown_layout_is_rejected(self):
        with self.assertRaises(ValueError):
            select_owned([{"window-id": 456, "app-pid": 123, "window-layout": "unknown"}], self.identity)

    @patch("native_window_setup.subprocess.run")
    def test_wrong_owner_never_receives_a_layout_command(self, run):
        run.return_value.returncode = 0
        run.return_value.stdout = '[{"window-id":456,"app-pid":999,"window-layout":"h_tiles"}]'
        run.return_value.stderr = ""
        with self.assertRaises(ValueError):
            WindowLease(self.identity, executable="/test/aerospace").prepare()
        self.assertEqual([call.args[0][1] for call in run.call_args_list], ["list-windows"])

    @patch("native_window_setup.subprocess.run")
    def test_valid_owner_is_floated_only_by_explicit_window_id(self, run):
        tiled = {"window-id": 456, "app-pid": 123, "window-layout": "h_tiles"}
        floating = {**tiled, "window-layout": "floating"}
        run.side_effect = [reply([tiled]), reply(), reply([floating])]
        lease = WindowLease(self.identity, executable="/test/aerospace")
        lease.prepare()
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[1], ["/test/aerospace", "layout", "--window-id", "456", "floating"])
        self.assertIn("--pid", commands[0])
        self.assertEqual(lease.audit["status"], "owned_window_floated")

    @patch("native_window_setup.subprocess.run")
    def test_closed_process_needs_no_restore_command(self, run):
        lease = WindowLease(self.identity, executable="/test/aerospace", changed=True)
        lease.restore(SimpleNamespace(pid=123, poll=lambda: 0))
        run.assert_not_called()
        self.assertEqual(lease.audit["cleanup"], "owned_process_exited")

    @patch("native_window_setup.subprocess.run")
    def test_restore_rejects_different_process_before_query(self, run):
        lease = WindowLease(self.identity, executable="/test/aerospace", changed=True)
        with self.assertRaises(ValueError):
            lease.restore(SimpleNamespace(pid=999, poll=lambda: None))
        run.assert_not_called()

    @patch("native_window_setup.subprocess.run")
    def test_restore_never_mutates_a_foreign_window(self, run):
        run.return_value = reply([{"window-id": 456, "app-pid": 999, "window-layout": "floating"}])
        lease = WindowLease(self.identity, executable="/test/aerospace", changed=True)
        with self.assertRaises(ValueError):
            lease.restore(SimpleNamespace(pid=123, poll=lambda: None))
        self.assertEqual([call.args[0][1] for call in run.call_args_list], ["list-windows"])


if __name__ == "__main__":
    unittest.main()
