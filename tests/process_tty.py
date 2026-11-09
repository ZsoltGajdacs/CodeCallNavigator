"""Run one production process-runner test under a real controlling terminal."""

import errno
import os
from pathlib import Path
import pty
import select
import signal
import subprocess
import sys
import time


def owned_analyzer(marker, session):
    if not marker.exists():
        return None
    try:
        identifier = int(marker.read_text())
        return identifier if os.getsid(identifier) == session else None
    except (ProcessLookupError, ValueError):
        return None


def terminate_group(identifier):
    if identifier is None:
        return
    try:
        os.killpg(identifier, signal.SIGTERM)
        os.killpg(identifier, signal.SIGCONT)
    except ProcessLookupError:
        pass


def read_terminal(master):
    if not select.select([master], [], [], 0.05)[0]:
        return
    try:
        data = os.read(master, 65536)
        sys.stdout.buffer.write(data)
        sys.stdout.buffer.flush()
    except OSError as error:
        if error.errno != errno.EIO:
            raise


def analyzer_stopped(marker, session):
    analyzer = owned_analyzer(marker, session)
    if analyzer is None:
        return False
    state = subprocess.run(["ps", "-o", "state=", "-p", str(analyzer)],
                           capture_output=True, text=True, check=False).stdout.strip()
    return state.startswith("T")


def supervise(identifier, master, marker):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        read_terminal(master)
        finished, status = os.waitpid(identifier, os.WNOHANG)
        if finished:
            return os.waitstatus_to_exitcode(status)
        if analyzer_stopped(marker, identifier):
            print("Analyzer child stopped on controlling-terminal input (SIGTTIN)", flush=True)
            return 1
    print("Process-runner controlling-terminal regression timed out", flush=True)
    return 1


def main():
    binary, test_name, directory = sys.argv[1:]
    marker = Path(directory) / "stdin-child.pid"
    identifier, master = pty.fork()
    if identifier == 0:
        environment = dict(os.environ, CODEGRAPH_TEST_PROCESS_TTY=directory)
        os.execve(binary, [binary, "--exact", test_name, "--nocapture", "--test-threads=1"], environment)
    try:
        return supervise(identifier, master, marker)
    finally:
        terminate_group(owned_analyzer(marker, identifier))
        terminate_group(identifier)
        try:
            os.waitpid(identifier, 0)
        except ChildProcessError:
            pass
        os.close(master)


if __name__ == "__main__":
    sys.exit(main())
