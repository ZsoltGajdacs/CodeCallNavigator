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
            if failed and not step.always:
                continue
            result = self.results[index]
            result["status"] = "running"
            result["log"] = f"{index + 1:02d}-{step.name}.log"
            self.save()
            print(f"[{index + 1}/{len(self.steps)}] {step.name}", flush=True)
            began = time.monotonic()
            with (self.output / result["log"]).open("w") as log:
                status, code = execute(step, self.root, log)
            result.update(status=status, exit_code=code,
                          seconds=round(time.monotonic() - began, 3))
            self.save()
            if status != "passed":
                print(f"{step.name}: {status}; inspect {self.output / result['log']}", flush=True)
                print((self.output / result["log"]).read_text(errors="replace")[-6000:])
                failed = True
        return int(failed)


def stop_group(process, log):
    """Return false if OS restrictions prevent proving descendant cleanup."""
    for sig in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            break
        except PermissionError:
            log.write("\nProcess-group cleanup unavailable; descendant cleanup is unverified.\n")
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
