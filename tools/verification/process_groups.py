"""Positive process-group absence proof after a reaped leader's signal denial."""
from dataclasses import dataclass
from enum import Enum
import os
import subprocess
import time


class GroupState(Enum):
    ABSENT = 'absent'
    UNVERIFIED = 'unverified'
    LEADER_LIVE = 'leader-live'
    MEMBERS_PRESENT = 'members-present'


@dataclass(frozen=True)
class GroupProbe:
    state: GroupState
    detail: str

    @property
    def absent(self):
        return self.state == GroupState.ABSENT


def parse_group_members(text, group, observer_pid):
    """Reject incomplete observations; every matching member, including zombies, counts."""
    seen, members = set(), []
    for line in text.splitlines():
        if not line.strip():
            continue
        pid, pgid = parse_process_row(line)
        if pid in seen:
            raise ValueError('Duplicate PID in native process table')
        seen.add(pid)
        if pgid == group:
            members.append(pid)
    if observer_pid not in seen:
        raise ValueError('Native process table omitted the observer')
    return tuple(sorted(members))


def parse_process_row(line):
    fields = line.split()
    if len(fields) != 2 or not all(field.isdecimal() for field in fields):
        raise ValueError('Malformed native process-table row')
    return tuple(map(int, fields))


def group_snapshot(group, timeout):
    try:
        result = subprocess.run(['/bin/ps', '-axo', 'pid=,pgid='], capture_output=True,
                                text=True, timeout=timeout, check=False)
        if result.returncode:
            return GroupProbe(GroupState.UNVERIFIED, f'native ps exit={result.returncode}')
        members = parse_group_members(result.stdout, group, os.getpid())
    except (OSError, ValueError, subprocess.TimeoutExpired) as error:
        return GroupProbe(GroupState.UNVERIFIED, f'native ps {type(error).__name__}: {str(error)[:200]}')
    if members:
        return GroupProbe(GroupState.MEMBERS_PRESENT, f'member_pids={members[:10]} total={len(members)}')
    return GroupProbe(GroupState.ABSENT, 'complete native process table contains no owned group members')


def confirm_group_absent(process):
    """Only a reaped leader plus a complete empty-group snapshot proves absence."""
    if process.pid <= 1:
        return GroupProbe(GroupState.UNVERIFIED, 'owned PGID must be greater than one')
    if process.poll() is None:
        return GroupProbe(GroupState.LEADER_LIVE, 'leader has not been reaped')
    deadline = time.monotonic() + 1.0
    probe = GroupProbe(GroupState.UNVERIFIED, 'absence deadline elapsed before a process-table observation')
    while True:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return probe
        probe = group_snapshot(process.pid, remaining)
        if probe.state != GroupState.MEMBERS_PRESENT:
            return probe
        time.sleep(min(0.025, max(0, deadline - time.monotonic())))


def signal_denial_detail(process, sig, error, probe):
    return (f'killpg PGID={process.pid} signal={sig.name} errno={error.errno}; '
            f'probe={probe.state.value}: {probe.detail}')
