"""Temporarily float only a PID-verified acceptance window managed by AeroSpace."""
from dataclasses import dataclass, field
from enum import Enum
import json
import shutil
import subprocess
import time


@dataclass(frozen=True)
class WindowIdentity:
    process_id: int
    window_id: int

    def __post_init__(self):
        if self.process_id <= 0 or self.window_id <= 0:
            raise ValueError("Window setup requires positive owned process/window IDs")


def select_owned(rows, identity):
    matches = [row for row in rows if row.get("window-id") == identity.window_id]
    if not matches:
        return None
    if len(matches) != 1 or matches[0].get("app-pid") != identity.process_id:
        raise ValueError("Window setup rejected an ambiguous or foreign process/window identity")
    if matches[0].get("window-layout") not in {
        "floating", "h_tiles", "v_tiles", "h_accordion", "v_accordion",
    }:
        raise ValueError("Window setup rejected an unknown AeroSpace layout")
    return matches[0]


class Mode(str, Enum):
    FLOATING = "floating"
    TILING = "tiling"


def mode(row):
    return Mode.FLOATING if row["window-layout"] == "floating" else Mode.TILING


def matches_state(row, expected):
    if row is None:
        return False
    return expected is None or mode(row) == expected


@dataclass
class WindowLease:
    identity: WindowIdentity
    executable: str | None = None
    original_layout: str | None = None
    changed: bool = False
    audit: dict = field(default_factory=dict)

    def prepare(self):
        self.audit.update(process_id=self.identity.process_id, window_id=self.identity.window_id)
        if self.executable is None:
            self.executable = shutil.which("aerospace")
        if self.executable is None:
            self.audit["status"] = "manager_not_installed"
            return
        before = self._wait_owned()
        self.original_layout = before["window-layout"]
        self.audit["before"] = before
        if mode(before) == Mode.FLOATING:
            self.audit["status"] = "already_floating"
            return
        self._layout(Mode.FLOATING)
        self.changed = True
        self.audit["after"] = self._wait_owned(Mode.FLOATING)
        self.audit["status"] = "owned_window_floated"

    def restore(self, process):
        if not self.changed:
            self.audit["cleanup"] = "layout_unchanged"
            return
        if process.poll() is not None:
            self.audit["cleanup"] = "owned_process_exited"
            return
        if process.pid != self.identity.process_id:
            raise ValueError("Cannot restore a different process's window")
        current = self._query_owned()
        if current is None:
            self.audit["cleanup"] = "owned_window_closed"
            return
        self._layout(Mode.TILING)
        self.audit["restored"] = self._wait_owned(Mode.TILING)
        self.audit["cleanup"] = "owned_window_returned_to_tiling"

    def _wait_owned(self, expected=None):
        deadline = time.monotonic() + 5.0
        while True:
            row = self._query_owned()
            if matches_state(row, expected):
                return row
            if time.monotonic() >= deadline:
                raise RuntimeError("Owned window did not enter the requested AeroSpace state")
            time.sleep(0.05)

    def _query_owned(self):
        result = self._command([
            "list-windows", "--monitor", "all", "--pid", str(self.identity.process_id),
            "--format", "%{window-id} %{app-pid} %{window-layout}", "--json",
        ])
        return select_owned(json.loads(result.stdout), self.identity)

    def _layout(self, selected):
        self._command(["layout", "--window-id", str(self.identity.window_id), selected.value])

    def _command(self, arguments):
        command = [self.executable, *arguments]
        self.audit.setdefault("commands", []).append(command)
        result = subprocess.run(command, capture_output=True, text=True, timeout=5)
        if result.returncode != 0:
            raise RuntimeError(f"Owned-window command failed: {result.stderr.strip()}")
        return result
