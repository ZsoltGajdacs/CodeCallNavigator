"""Bounded cleanup leases cover only query-session groups spawned by this owned app."""
from dataclasses import asdict, dataclass, field
import os
import signal
import subprocess
import time

from csharp_flow_acceptance import require


@dataclass(frozen=True)
class Process:
    pid: int
    parent: int
    group: int
    started: str
    command: str
    state: str = field(default="", compare=False)


def processes():
    result = subprocess.run(["/bin/ps", "-ww", "-axo", "pid=,ppid=,pgid=,stat=,lstart=,command="],
                            capture_output=True, text=True, timeout=5, check=True)
    rows = []
    for line in result.stdout.splitlines():
        fields = line.split(None, 9)
        if len(fields) == 10:
            rows.append(Process(*(int(value) for value in fields[:3]), " ".join(fields[4:9]), fields[9], fields[3]))
    return rows


def session_command(command, engine):
    expected = str(engine) + " query-session"
    return command == expected or command.endswith(" " + expected)


def query_leases(app, engine):
    require(app.poll() is None, "Cannot discover owned queries after the parent exited")
    owned = [row for row in processes() if row.parent == app.pid and session_command(row.command, engine)]
    require(all(row.group == row.pid for row in owned), "Owned query process-group inspection is inconclusive")
    return owned


def group_alive(group):
    result = subprocess.run(["/bin/ps", "-axo", "pgid="], capture_output=True,
                            text=True, timeout=5, check=True)
    groups = [int(line) for line in result.stdout.splitlines() if line.strip()]
    require(groups and all(value >= 0 for value in groups), "Process-group membership inspection is inconclusive")
    return group in groups


def wait_groups(leases, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if not any(group_alive(lease.group) for lease in leases):
            return True
        time.sleep(0.02)
    return not any(group_alive(lease.group) for lease in leases)


def stop_verified(lease, number):
    current = next((row for row in processes() if row.pid == lease.pid), None)
    if not group_alive(lease.group):
        return
    require(current == lease, "Owned query PID/group identity changed; refusing a signal")
    try:
        os.killpg(lease.group, number)
    except ProcessLookupError:
        return


def group_members(leases):
    result = subprocess.run(["/bin/ps", "-axo", "pid=,ppid=,pgid=,stat="],
                            capture_output=True, text=True, timeout=5, check=True)
    groups = {lease.group for lease in leases}
    members = []
    for line in result.stdout.splitlines():
        fields = line.split()
        require(len(fields) == 4, "Incomplete process-state snapshot")
        pid, parent, group = map(int, fields[:3])
        if group in groups:
            state = fields[3]
            members.append({"pid": pid, "ppid": parent, "pgid": group, "state": state})
    return members


def termination_snapshot(leases):
    members = group_members(leases)
    if not members:
        return {"status": "absent", "members": []}
    if any(row["state"][0] not in "IRSTUZDWL" for row in members):
        return {"status": "unknown", "members": members}
    status = "awaiting_parent_reap" if all(row["state"].startswith("Z") for row in members) else "running"
    return {"status": status, "members": members}


def wait_terminated(leases, timeout, audit):
    deadline = time.monotonic() + timeout
    while True:
        snapshot = termination_snapshot(leases)
        audit["snapshots"].append(snapshot)
        require(snapshot["status"] != "unknown", "Unknown leased process state")
        if snapshot["status"] != "running" or time.monotonic() >= deadline:
            return snapshot
        time.sleep(0.02)


def cleanup_queries(app, engine, disarm, audit=None):
    audit = {} if audit is None else audit
    audit.update(status="observing", leases=[], snapshots=[], signals=[])
    leases = query_leases(app, engine)
    audit["leases"] = [asdict(lease) for lease in leases]
    disarm()
    snapshot = wait_terminated(leases, 0.5, audit)
    if snapshot["status"] == "running":
        terminate_running(leases, snapshot, audit)
        snapshot = wait_terminated(leases, 2.0, audit)
    require(snapshot["status"] != "running", "Owned query group did not terminate after verified SIGTERM")
    audit["status"] = snapshot["status"]
    return audit


def terminate_running(leases, snapshot, audit):
    live_groups = {row["pgid"] for row in snapshot["members"] if not row["state"].startswith("Z")}
    for lease in leases:
        if lease.group in live_groups:
            stop_verified(lease, signal.SIGTERM)
            audit["signals"].append(lease.pid)


@dataclass
class OwnedRun:
    app: object
    engine: object
    gate: object
    directory: object
    lease: object = None

    def disarm(self):
        # Expansion has no injected Git gate; query ownership is identical.
        if self.gate is not None:
            self.gate.disarm()

    def prepare_parent_close(self, audit):
        if self.app.poll() is None:
            cleanup_queries(self.app, self.engine, self.disarm, audit)
        else:
            self.disarm()
            audit.update(status="parent-exited", leases=[], snapshots=[], signals=[])

    def require_released(self, audit):
        leases = [Process(**value) for value in audit["pre_parent"].get("leases", [])]
        before = group_members(leases)
        audit["post_parent"] = {"status": "observing", "before": before, "members": before}
        require(wait_groups(leases, 2.0), "Owned query group remains after its parent exited")
        members = group_members(leases)
        audit["post_parent"]["members"] = members
        require(not members, "Leased group membership changed during final inspection")
        if self.gate is not None and self.gate.observed is not None:
            require(not group_alive(self.gate.observed["group"]), "Previously owned gate/query group remains after parent cleanup")
        require(not any(session_command(row.command, self.engine) for row in processes()),
                "A private relocated query session remains; ownership cleanup cannot be certified")
        audit["post_parent"]["status"] = "cleared"

    def inconclusive(self, audit, stage, error):
        audit.update(status="inconclusive", diagnostics_preserved=True)
        audit.setdefault("errors", []).append({"stage": stage, "message": str(error)})
        audit["error"] = str(error)
        try:
            self.disarm()
        except Exception as disarm_error:
            audit["disarm_error"] = str(disarm_error)

    def close(self):
        from csharp_desktop_acceptance import cleanup_native, write_json
        audit = {"status": "observing", "pre_parent": {}}
        failures = []
        try:
            self.prepare_parent_close(audit["pre_parent"])
        except Exception as error:
            self.inconclusive(audit, "pre_parent", error)
            failures.append(error)
        try:
            cleanup_native(self.app, self.lease, self.directory)
        except Exception as error:
            self.inconclusive(audit, "parent_close", error)
            failures.append(error)
        try:
            self.require_released(audit)
        except Exception as error:
            self.inconclusive(audit, "post_parent", error)
            failures.append(error)
        if not failures:
            audit["status"] = "cleared"
        write_json(self.directory / "query-cleanup.json", audit)
        if failures:
            fixture = self.gate.project if self.gate is not None else self.directory
            raise RuntimeError(f"Owned native cleanup incomplete; preserve fixture {fixture}: {audit['error']}") from failures[0]
        return audit
