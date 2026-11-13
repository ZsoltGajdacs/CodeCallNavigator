"""Bounded subprocess/resource observations and statistical reporting."""
import json
import math
import os
import resource
import signal
import statistics
import subprocess
import tempfile
import threading
import time

from measure_analysis import process_tree_bytes


def distribution(values):
    if not values:
        return None
    ordered = sorted(values)
    return {"count": len(ordered), "median": statistics.median(ordered), "p95": ordered[math.ceil(len(ordered) * .95) - 1], "maximum": ordered[-1]}


def process_inventory():
    output = subprocess.check_output(["/bin/ps", "-axo", "pid=,ppid=,lstart="], text=True, timeout=5)
    return [(int(pid), int(parent), started) for pid, parent, started in
            (line.split(maxsplit=2) for line in output.splitlines())]


def descendants(pid):
    processes = process_inventory()
    selected, pending = set(), {pid}
    while pending:
        selected.update(pending)
        pending = {child for child, parent, _ in processes if parent in pending} - selected
    return {child: started for child, _, started in processes if child in selected and child != pid}


def inventory_owned(process, owned, errors):
    try:
        owned.update(descendants(process.pid))
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        errors.append(f"Cannot inventory descendants: {error}")


def kill_owned(owned, errors):
    try:
        current = {pid: started for pid, _, started in process_inventory()}
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        errors.append(f"Cannot verify descendant identities: {error}")
        return
    for pid, started in owned.items():
        if current.get(pid) == started:
            try:
                os.kill(pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def signal_group(process, sig):
    if process.poll() is None:
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            pass


def stop_owned(process, grace):
    """Let the engine reap separate analyzer groups before forceful cleanup."""
    errors, owned = [], {}
    inventory_owned(process, owned, errors)
    signal_group(process, signal.SIGTERM)
    try:
        process.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        inventory_owned(process, owned, errors)
        kill_owned(owned, errors)
        signal_group(process, signal.SIGKILL)
    finally:
        process.wait()
    # An exited root can still leave separately grouped workers behind.
    kill_owned(owned, errors)
    return errors


class ProcessMeasurement:
    """Own one process, its sampling thread and bounded cancellation lifecycle."""
    def __init__(self, command, stdout, stderr):
        self.stdout, self.stderr = stdout, stderr
        self.samples, self.sampling_errors, self.cleanup_errors = [], [], []
        self.stopped = threading.Event()
        self.before = resource.getrusage(resource.RUSAGE_CHILDREN)
        self.started = time.monotonic()
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=stdout, stderr=stderr, start_new_session=True)
        self.sampler = threading.Thread(target=self.sample)
        self.sampler.start()
        self.timed_out = False

    def sample(self):
        while not self.stopped.wait(.05):
            try:
                self.samples.append(process_tree_bytes(self.process.pid))
            except (OSError, ValueError, subprocess.SubprocessError) as error:
                self.sampling_errors.append(str(error))

    def send(self, request, keep_open):
        if request is not None:
            self.process.stdin.write((json.dumps(request) + '\n').encode())
            self.process.stdin.flush()
        if not keep_open:
            self.process.stdin.close()

    def execute(self, request, timeout, keep_open):
        try:
            self.send(request, keep_open)
            self.process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            self.timed_out = True
            self.cleanup_errors = stop_owned(self.process, 5)
        except BaseException:
            stop_owned(self.process, 5)
            raise
        finally:
            self.finish()

    def close_input(self):
        if not self.process.stdin.closed:
            try:
                self.process.stdin.close()
            except BrokenPipeError:
                pass

    def finish(self):
        self.close_input()
        self.stopped.set()
        self.sampler.join()

    def resource_report(self):
        """Resource totals after reaping; streaming callers own their output reader."""
        elapsed = time.monotonic() - self.started
        after = resource.getrusage(resource.RUSAGE_CHILDREN)
        return {"milliseconds": elapsed * 1000, "exit_code": self.process.returncode, "timed_out": self.timed_out,
                "cpu_seconds": after.ru_utime + after.ru_stime - self.before.ru_utime - self.before.ru_stime,
                "sampled_peak_tree_rss_bytes": max(self.samples) if self.samples else None,
                "resource_samples": len(self.samples), "sampling_errors": self.sampling_errors, "cleanup_errors": self.cleanup_errors}

    def report(self):
        self.stdout.seek(0)
        self.stderr.seek(0)
        return self.resource_report() | {
            "stdout": self.stdout.read().decode(errors="replace"),
            "stderr": self.stderr.read().decode(errors="replace")}


def run(command, request=None, timeout=120, keep_open=False):
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        measurement = ProcessMeasurement(command, stdout, stderr)
        measurement.execute(request, timeout, keep_open)
        return measurement.report()


def response(observation):
    if observation["exit_code"]:
        raise RuntimeError(observation["stderr"][-4000:] or observation["stdout"][-4000:])
    return json.loads(observation["stdout"])


def measured_distribution(records, field, section=None):
    observations = [record[section] if section else record for record in records]
    return distribution([observation[field] for observation in observations if observation[field] is not None])


def summarize_group(group):
    warm = group["warm"]
    successful = group["cold"] + warm
    return {"cold_records": len(group["cold"]), "warm_records": len(warm), "failed_records": len(group["failures"]),
            "detection_ms": measured_distribution(warm, "milliseconds", "detection"),
            "refresh_ms": measured_distribution(warm, "milliseconds", "refresh"),
            "adoption_ms": measured_distribution(warm, "milliseconds", "adoption"),
            "save_to_adoption_ms": measured_distribution(warm, "save_to_adoption_ms"),
            "refresh_cpu_seconds": measured_distribution(warm, "cpu_seconds", "refresh"),
            "refresh_sampled_peak_tree_rss_bytes": measured_distribution(warm, "sampled_peak_tree_rss_bytes", "refresh"),
            "equivalent": all(record["equivalent"] for record in successful) if successful else None}


def summarize(records):
    groups = {}
    for record in records:
        key = f'{record["language"]}/{record["scenario"]}'
        group = groups.setdefault(key, {"cold": [], "warm": [], "failures": []})
        group["failures" if "failure" in record else record["phase"]].append(record)
    return {key: summarize_group(group) for key, group in groups.items()}
