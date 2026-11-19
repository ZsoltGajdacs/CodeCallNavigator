"""Bounded subprocess stages with persistent, truthful verification results."""
from dataclasses import dataclass, field
from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import signal
import subprocess
import time
if __package__:
    from .desktop_session import DEFERRED_DESKTOP_EXITS
    from .process_groups import confirm_group_absent, signal_denial_detail
else:
    from desktop_session import DEFERRED_DESKTOP_EXITS
    from process_groups import confirm_group_absent, signal_denial_detail


RESULT_LABELS = {0: "passed", 1: "failed", 3: "incomplete"}


def result_code(results):
    if any(row.get("interrupted") or row["status"] not in ("passed", "not-run") for row in results):
        return 1
    if any(row["status"] == "not-run" for row in results):
        return 3
    return 0


@contextmanager
def exclusive(root):
    """Do not run competing builds, snapshots or packaging stages in one checkout."""
    path = Path(root) / "target/verification.lock"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a+") as guard:
        try:
            fcntl.flock(guard, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise RuntimeError("Another verification run owns this checkout; wait for it to finish") from error
        yield


@dataclass
class Step:
    name: str
    command: list[str]
    timeout: int = 600
    environment: dict[str, str] = field(default_factory=dict)
    always: bool = False
    requires_desktop: bool = False

    def deferred_reason(self, status, code):
        if self.requires_desktop and status == "failed":
            return DEFERRED_DESKTOP_EXITS.get(code)
        return None


class Runner:
    def __init__(self, root, output, steps):
        self.root, self.output, self.steps = Path(root), Path(output), steps
        self.results = [dict(name=s.name, command=s.command, timeout=s.timeout,
                             status="not-run") for s in steps]

    def save(self):
        (self.output / "results.json").write_text(json.dumps(self.results, indent=2) + "\n")

    def run(self):
        self.output.mkdir(parents=True, exist_ok=False)
        self.save()
        failed = False
        for index, step in enumerate(self.steps):
            try:
                if failed and not step.always:
                    continue
                status = self.run_step(index, step)
                failed = failed or status not in ("passed", "not-run")
            except KeyboardInterrupt:
                self.record_interruption(index)
                failed = True
        return result_code(self.results)

    def run_step(self, index, step):
        result = self.results[index]
        result["status"] = "running"
        result["log"] = f"{index + 1:02d}-{step.name}.log"
        self.save()
        print(f"[{index + 1}/{len(self.steps)}] {step.name}", flush=True)
        began = time.monotonic()
        with (self.output / result["log"]).open("w") as log:
            status, code = execute(step, self.root, log)
        reason = step.deferred_reason(status, code)
        if reason:
            status = "not-run"
            result["reason"] = reason
        result.update(status=status, exit_code=code,
                      seconds=round(time.monotonic() - began, 3))
        self.save()
        self.print_stage_outcome(result)
        return status

    def print_stage_outcome(self, result):
        name, status = result["name"], result["status"]
        if status == "not-run":
            print(f"{name}: not performed ({result['reason']})", flush=True)
        elif status != "passed":
            log = self.output / result["log"]
            print(f"{name}: {status}; inspect {log}", flush=True)
            print(log.read_text(errors="replace")[-6000:])

    def record_interruption(self, index):
        result = self.results[index]
        result['interrupted'] = True
        if result['status'] in ('running', 'not-run'):
            result.update(status='interrupted', exit_code=None)
        # A signal after a completed stage must not erase that stage's actual
        # outcome. The interrupted run still fails and skips later ordinary work.
        self.save()


def stop_group(process, log):
    """Return false if OS restrictions prevent proving descendant cleanup."""
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        except PermissionError as error:
            probe = confirm_group_absent(process)
            detail = signal_denial_detail(process, sig, error, probe)
            log.write(f"\nProcess-group signal denied: {detail}\n")
            if probe.absent:
                break
            log.write("Process-group cleanup unavailable; descendant cleanup is unverified.\n")
            process.kill()
            process.wait()
            return False
        if sig == signal.SIGTERM:
            time.sleep(0.2)
    process.wait()
    return True


def execute(step, root, log):
    try:
        process = subprocess.Popen(step.command, cwd=root, stdin=subprocess.DEVNULL,
                                   stdout=log, stderr=subprocess.STDOUT,
                                   env=os.environ | step.environment, start_new_session=True)
    except OSError as error:
        log.write(f"Cannot start stage: {error}\n")
        return "failed", None
    try:
        code = process.wait(timeout=step.timeout)
        if code and not stop_group(process, log):
            return "cleanup-unavailable", code
        return ("passed" if code == 0 else "failed"), code
    except subprocess.TimeoutExpired:
        log.write(f"\nStage timed out after {step.timeout} seconds.\n")
        cleaned = stop_group(process, log)
        return ("timed-out" if cleaned else "cleanup-unavailable"), process.returncode
    except KeyboardInterrupt:
        cleaned = stop_group(process, log)
        log.write("\nVerification interrupted.\n")
        return ("interrupted" if cleaned else "cleanup-unavailable"), process.returncode
