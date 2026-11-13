#!/usr/bin/env python3
"""Real TypeScript/compiler acceptance for the opt-in CLI refresh coordinator.

All edits, locks and compiler outputs belong to disposable projects. Process
cleanup is checked before the measurement helper's emergency cleanup can run.
This is correctness acceptance, not a latency benchmark or a fake analyzer.
"""
import argparse
import array
import fcntl
import gzip
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import termios
import time

from freshness_measurement import descendants, kill_owned, process_inventory, response, run
from freshness_observer_measurement import Observer, request
from freshness_scenarios import Project
from measure_freshness import Session, metadata


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def activity(event):
    return event.get("activity", {})


def active_identities(owned):
    current = {pid: started for pid, _, started in process_inventory()}
    matching = [str(pid) for pid, started in owned.items() if current.get(pid) == started]
    if not matching:
        return []
    statuses = subprocess.run(["/bin/ps", "-p", ",".join(matching), "-o", "pid=,stat="],
                              capture_output=True, text=True, timeout=5)
    return [int(line.split()[0]) for line in statuses.stdout.splitlines()
            if not line.split()[1].startswith("Z")]


class Monitor:
    def __init__(self, case, automatic=True):
        self.case, self.events, self.report = case, [], None
        command = [str(case.executable), "observe"]
        if automatic:
            command.append("--auto-refresh")
        self.observer = Observer(command, request(case.project), case.timeout)

    def next(self, deadline):
        received, event = self.observer.next_event(deadline)
        self.events.append({"received": received, "event": event})
        return event

    def until(self, predicate, timeout=None):
        deadline = time.monotonic() + (timeout or self.case.timeout)
        while True:
            event = self.next(deadline)
            if predicate(event):
                return event

    def state(self, *states):
        return self.until(lambda event: activity(event).get("state") in states)

    def quiet(self, seconds=5.2):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            try:
                self.next(deadline)
            except RuntimeError:
                if time.monotonic() >= deadline:
                    return
                raise

    def runs(self):
        # Activity frames are coalesced. The retained monotonic counter survives
        # omitted Running notifications and is the only analysis-count oracle.
        counts = [activity(entry["event"])["runs_started"] for entry in self.events
                  if "runs_started" in activity(entry["event"])]
        require(bool(counts), "Coordinator omitted its retained attempt counter")
        require(counts == sorted(counts), "Coordinator attempt counter regressed")
        return max(counts)

    def publish(self, generation):
        event = self.until(lambda frame: frame.get("observation", {}).get("generation") == generation
                           and frame["observation"]["input_status"] != "stale")
        require(event["observation"]["contract_valid"], "Published artifact contract is invalid")
        return event

    def cancel_refresh(self):
        self.observer.measurement.send({"version": 1, "command": "cancel_refresh"}, keep_open=True)

    def node_worker(self):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            owned = descendants(self.observer.measurement.process.pid)
            output = subprocess.check_output(["/bin/ps", "-axo", "pid=,command="], text=True)
            for line in output.splitlines():
                fields = line.strip().split(maxsplit=1)
                if len(fields) != 2:
                    continue
                pid, command = fields
                if int(pid) not in owned:
                    continue
                if "analyze.js" in command and " worker " in command:
                    return int(pid)
            time.sleep(.01)
        raise AssertionError("No real TypeScript compiler worker was observed")

    def close(self, mode="eof"):
        if self.report is not None:
            return self.report
        process = self.observer.measurement.process
        owned = descendants(process.pid) if process.poll() is None else {}
        started = time.monotonic()
        try:
            self.shutdown(mode)
            code = process.wait(timeout=15)
            require(code == 0, f"Unexpected shutdown exit {code}")
            require(not active_identities(owned), "Coordinator left active analysis descendants")
        finally:
            # Failures remain failures even though the helper subsequently reaps.
            self.report = self.observer.close()
            kill_owned(owned, self.report["cleanup_errors"])
        self.report["natural_shutdown_milliseconds"] = (time.monotonic() - started) * 1000
        self.report["shutdown_mode"] = mode
        require(not self.report["cleanup_errors"], f"Cleanup errors: {self.report}")
        return self.report

    def shutdown(self, mode):
        process = self.observer.measurement.process
        if process.poll() is not None:
            return
        if mode == "sigterm":
            process.send_signal(signal.SIGTERM)
        elif mode == "broken_pipe":
            self.observer.stop_reader.set()
            self.observer.reader.join(timeout=1)
            require(not self.observer.reader.is_alive(), "Reader did not release stdout")
            process.stdout.close()
        else:
            self.observer.measurement.close_input()


class Case:
    def __init__(self, arguments, name):
        self.arguments, self.name = arguments, name
        self.executable, self.timeout = arguments.executable, arguments.timeout
        self.directory = tempfile.TemporaryDirectory(prefix=f"ccn-auto-accept-{name}-")
        self.project = Project(Path(self.directory.name), "typescript", arguments.typescript_modules,
                               Path(__file__).resolve().parents[1])
        self.project.reset()
        self.session = Session(self.executable, self.project, self.timeout)
        self.baseline = self.session.analyze()
        self.artifact = self.project.root / ".codecallnavigator/graph.json"
        self.before = self.artifact.read_bytes()
        self.monitors = []

    def monitor(self, automatic=True):
        monitor = Monitor(self, automatic)
        self.monitors.append(monitor)
        initial = monitor.next(time.monotonic() + self.timeout)
        require(initial.get("kind") == "initial" and initial.get("sequence") == 1,
                "Observation stream must begin with its initial generation-bound observation")
        return monitor

    def edit(self, value):
        self.project.source.write_text(self.project.text(value))

    def reference(self, value):
        with tempfile.TemporaryDirectory(prefix="ccn-auto-accept-reference-") as directory:
            project = Project(Path(directory), "typescript", self.project.modules,
                              self.project.repository)
            project.reset()
            project.source.write_text(project.text(value))
            complete = Session(self.executable, project, self.timeout).analyze()
            raw = (project.root / ".codecallnavigator/graph.json").read_bytes()
            return complete["generation"], raw

    def unchanged(self):
        require(self.artifact.read_bytes() == self.before, "Previous successful artifact changed")

    def equivalent(self, expected):
        actual = self.artifact.read_bytes()
        if actual != expected:
            self.arguments.output.joinpath(f"{self.name}-actual.json.gz").write_bytes(gzip.compress(actual, mtime=0))
            self.arguments.output.joinpath(f"{self.name}-expected.json.gz").write_bytes(gzip.compress(expected, mtime=0))
            raise AssertionError("Automatic publication differs from independent clean analysis")

    def finish(self):
        errors = []
        for monitor in reversed(self.monitors):
            try:
                monitor.close()
            except (AssertionError, OSError, RuntimeError, subprocess.SubprocessError) as error:
                errors.append(str(error))
        events = [{"events": monitor.events, "resources": monitor.report} for monitor in self.monitors]
        self.arguments.output.joinpath(f"{self.name}-events.json").write_text(json.dumps(events, indent=2) + "\n")
        self.directory.cleanup()
        require(not errors, f"Monitor cleanup failed: {errors}")


def burst_latest(case):
    generation, expected = case.reference(11)
    monitor = case.monitor()
    for value in range(2, 12):
        case.edit(value)
        time.sleep(.005)
    monitor.publish(generation)
    case.equivalent(expected)
    monitor.quiet()
    require(monitor.runs() == 1, f"Burst produced {monitor.runs()} analyses")


def invalid_then_repair(case):
    generation, expected = case.reference(2)
    monitor = case.monitor()
    case.project.source.write_text("export function broken(\n")
    monitor.state("failed")
    case.unchanged()
    runs = monitor.runs()
    monitor.quiet()
    require(monitor.runs() == runs, "Unchanged invalid inputs were retried")
    case.edit(2)
    monitor.publish(generation)
    case.equivalent(expected)


def manual_lock_contention(case):
    generation, expected = case.reference(2)
    monitor = case.monitor()
    with case.artifact.with_name("graph.lock").open("a+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        case.edit(2)
        monitor.state("busy")
        case.unchanged()
    monitor.publish(generation)
    case.equivalent(expected)


def coordinator_contention(case):
    first = case.monitor()
    first.state("watching")
    second = case.monitor()
    second.state("busy", "unavailable")
    second.quiet()
    require(not second.runs(), "A second coordinator acquired an occupied lease")
    case.unchanged()
    second.close()
    first.close()
    generation, expected = case.reference(2)
    next_owner = case.monitor()
    case.edit(2)
    next_owner.publish(generation)
    case.equivalent(expected)


def generated_outputs(case):
    monitor = case.monitor()
    paths = ["target/intermediate.bin", "build/Intermediate.class",
             ".codecallnavigator/.graph-publish-acceptance", ".codecallnavigator/inference.sqlite3"]
    for name in paths:
        path = case.project.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"generated output")
    monitor.quiet()
    case.unchanged()
    require(not monitor.runs(), "Generated outputs triggered automatic analysis")


def user_cancel(case):
    generation, expected = case.reference(3)
    monitor = case.monitor()
    case.edit(2)
    monitor.state("running")
    monitor.cancel_refresh()
    cancelled = monitor.state("cancelled")
    require(activity(cancelled).get("reason") == "user_cancelled", "User cancellation lost its reason")
    case.unchanged()
    runs = monitor.runs()
    monitor.quiet()
    require(monitor.runs() == runs, "User-cancelled input was retried")
    case.edit(3)
    monitor.publish(generation)
    case.equivalent(expected)


def active_shutdown(case, mode):
    monitor = case.monitor()
    case.edit(2)
    monitor.state("running")
    monitor.node_worker()
    monitor.close(mode)


def unread_output_bytes(monitor):
    pending = array.array("i", [0])
    fcntl.ioctl(monitor.observer.measurement.process.stdout.fileno(), termios.FIONREAD, pending, True)
    return pending[0]


def write_pressure_files(paths, value):
    for index, path in enumerate(paths):
        path.write_text(f"export function pressure{index}(){{return {value};}}\n")


def undrained_output(case):
    paths = [case.project.root / (f"pressure_{index:02d}_" + "observed_" * 6 + ".ts")
             for index in range(32)]
    write_pressure_files(paths, 1)
    case.baseline = case.session.analyze()
    case.before = case.artifact.read_bytes()
    monitor = case.monitor()
    monitor.observer.stop_reader.set()
    monitor.observer.reader.join(timeout=2)
    require(not monitor.observer.reader.is_alive(), "Cannot suspend client output draining")
    for value in range(2, 26):
        write_pressure_files(paths, value)
        time.sleep(.26)
    pending = unread_output_bytes(monitor)
    write_pressure_files(paths, 26)
    time.sleep(.6)
    require(pending >= 8192 and unread_output_bytes(monitor) == pending,
            f"Did not establish stable output backpressure: {pending} queued bytes")
    monitor.node_worker()
    report = monitor.close()
    report["undrained_output_bytes_before_shutdown"] = pending


def edits_during_analysis(case):
    generation, expected = case.reference(3)
    monitor = case.monitor()
    case.edit(2)
    monitor.state("running")
    worker = monitor.node_worker()
    os.kill(worker, signal.SIGSTOP)
    try:
        case.edit(3)
        time.sleep(.3)
        case.unchanged()
    finally:
        try:
            os.kill(worker, signal.SIGCONT)
        except ProcessLookupError:
            pass
    monitor.publish(generation)
    case.equivalent(expected)
    # Assertions cover the retained artifact while the compiler was paused and
    # the exact final generation. Coalesced activity is not a publication journal
    # and cannot prove that no unobserved transient replacement occurred.


def legacy_selection(case):
    artifact = json.loads(case.before)
    artifact["graph"]["analysis"].pop("selection")
    case.artifact.write_text(json.dumps(artifact))
    before = case.artifact.read_bytes()
    monitor = case.monitor()
    monitor.state("unavailable")
    case.edit(2)
    monitor.quiet()
    require(not monitor.runs(), "Legacy artifact caused guessed build selection")
    require(case.artifact.read_bytes() == before, "Legacy artifact was overwritten")


def future_selection(case):
    artifact = json.loads(case.before)
    artifact["graph"]["analysis"]["selection"]["version"] = 99
    case.artifact.write_text(json.dumps(artifact))
    before = case.artifact.read_bytes()
    queried = response(case.session.query())
    require(not queried.get("error"), "Future selection broke ordinary graph queries")
    monitor = case.monitor()
    monitor.state("unavailable")
    case.edit(2)
    monitor.quiet()
    require(not monitor.runs(), "Future selection caused guessed build selection")
    require(case.artifact.read_bytes() == before, "Future selection was overwritten")


def future_observation_version(case):
    initial = request(case.project)
    initial["query"]["observation_version"] = 2
    result = run([str(case.executable), "observe", "--auto-refresh"], initial,
                 timeout=5, keep_open=True)
    frames = [json.loads(line) for line in result["stdout"].splitlines()]
    require(result["exit_code"] == 2, "Future observation version did not fail explicitly")
    require(any(frame.get("error", {}).get("code") == "unsupported_freshness_version"
                for frame in frames), "Future version omitted its compatibility error")
    case.unchanged()


CASES = {
    "burst-latest": burst_latest,
    "invalid-repair": invalid_then_repair,
    "manual-lock": manual_lock_contention,
    "coordinator-lease": coordinator_contention,
    "generated-output": generated_outputs,
    "user-cancel": user_cancel,
    "eof": lambda case: active_shutdown(case, "eof"),
    "sigterm": lambda case: active_shutdown(case, "sigterm"),
    "broken-pipe": lambda case: active_shutdown(case, "broken_pipe"),
    "undrained-output": undrained_output,
    "concurrent-edit": edits_during_analysis,
    "legacy-selection": legacy_selection,
    "future-selection": future_selection,
    "future-version": future_observation_version,
}


def execute(arguments):
    arguments.executable = arguments.executable.resolve(strict=True)
    arguments.typescript_modules = arguments.typescript_modules.resolve(strict=True)
    arguments.output.mkdir(parents=True, exist_ok=False)
    capabilities = response(run([str(arguments.executable), "capabilities"]))
    require(capabilities.get("automatic_refresh", {}).get("version") == 1,
            "Configured CLI does not advertise automatic refresh v1")
    info = metadata(arguments.executable, Path(__file__).resolve().parents[1])
    info["capabilities"] = capabilities
    info.update(purpose="Real compiler/filesystem/process correctness acceptance; no latency benchmark",
                detection="Delivered observations and retained activity counters, not an event journal",
                idle_monitor="No isolated idle CPU measurement; resource totals include compiler journeys",
                reset_protocol="Independent disposable baseline per case; clean-reference artifacts for successful publications")
    arguments.output.joinpath("metadata.json").write_text(json.dumps(info, indent=2) + "\n")
    results = {}
    for name in arguments.cases:
        case = None
        try:
            case = Case(arguments, name)
            CASES[name](case)
            results[name] = {"passed": True}
        except (AssertionError, OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
            results[name] = {"passed": False, "failure": str(error)}
        finally:
            if case is not None:
                try:
                    case.finish()
                except (AssertionError, OSError, RuntimeError, subprocess.SubprocessError) as error:
                    results[name].update(passed=False, cleanup_failure=str(error))
        print(name, results[name], flush=True)
        arguments.output.joinpath("results.json").write_text(json.dumps(results, indent=2) + "\n")
    return int(any(not result["passed"] for result in results.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--typescript-modules", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--cases", type=lambda value: value.split(","), default=list(CASES))
    args = parser.parse_args()
    if set(args.cases) - CASES.keys() or args.timeout <= 0:
        parser.error("Invalid cases or timeout")
    raise SystemExit(execute(args))


if __name__ == "__main__":
    main()
