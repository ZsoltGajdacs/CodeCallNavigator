#!/usr/bin/env python3
"""Measure idle CLI observation after a real baseline analysis, without edits."""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import time

from freshness_observer_measurement import Observer, request
from freshness_scenarios import Project
from measure_automatic_refresh import analysis_resources, trace_event
from measure_freshness import Session, metadata
from measure_freshness_observer import assert_clean_shutdown, hardware, project_size


def parse_cpu_time(value):
    """Parse ps cumulative process CPU time; these short-lived processes have no days."""
    return sum(float(part) * 60 ** index for index, part in enumerate(reversed(value.strip().split(":"))))


def cpu_time(process):
    if process.poll() is not None:
        raise RuntimeError("Idle observer exited before its CPU sample")
    value = subprocess.check_output(["/bin/ps", "-p", str(process.pid), "-o", "time="],
                                    text=True, timeout=5).strip()
    return value, parse_cpu_time(value)


def ready(observer, mode, generation):
    observer.wait(lambda observation: observation["generation"] == generation)
    if mode == "passive":
        return
    deadline = time.monotonic() + observer.timeout
    while True:
        _, event = observer.next_event(deadline)
        validate_idle_event(event, generation)
        if event.get("activity", {}).get("state") == "watching":
            return


def idle_events(observer, seconds, generation):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        try:
            _, event = observer.next_event(deadline)
        except RuntimeError:
            if time.monotonic() >= deadline:
                return
            raise
        validate_idle_event(event, generation)


def validate_idle_event(event, generation):
    activity = event.get("activity", {})
    if activity.get("state") in ("unavailable", "failed", "cancelled"):
        raise RuntimeError(f"Idle monitor stopped functioning: {event}")
    if activity.get("runs_started", 0) != 0:
        raise RuntimeError(f"Idle monitor started an analysis: {event}")
    if event.get("observation", {}).get("generation", generation) != generation:
        raise RuntimeError(f"Idle artifact generation changed: {event}")


def idle_window(observer, seconds, generation):
    measurement = observer.measurement
    cpu_started = time.monotonic()
    before_text, before = cpu_time(measurement.process)
    sample_start, started = len(measurement.samples), time.monotonic()
    idle_events(observer, seconds, generation)
    elapsed = time.monotonic() - started
    sample_end = len(measurement.samples)
    after_text, after = cpu_time(measurement.process)
    cpu_elapsed = time.monotonic() - cpu_started
    memory = measurement.samples[sample_start:sample_end]
    return {"seconds": elapsed, "cpu_time_before": before_text, "cpu_time_after": after_text,
            "cli_self_cpu_seconds": after - before,
            "cpu_endpoint_elapsed_seconds": cpu_elapsed,
            "cpu_endpoint_overhead_seconds": cpu_elapsed - elapsed,
            "ps_cpu_resolution_seconds": .01,
            "mean_percent_one_core": (after - before) / cpu_elapsed * 100,
            "rss_samples": len(memory), "sampled_peak_tree_rss_bytes": max(memory, default=None),
            "mean_sampled_tree_rss_bytes": statistics.mean(memory) if memory else None}


def measure_mode(session, arguments, mode, generation):
    command = [str(session.executable), "observe"]
    if mode == "automatic":
        command.append("--auto-refresh")
    path = arguments.output / f"{session.project.language}-{mode}-stream.jsonl"
    with path.open("w") as output:
        observer = Observer(command, request(session.project), arguments.timeout,
                            on_event=lambda received, event: trace_event(output, received, event))
        try:
            ready(observer, mode, generation)
            result = idle_window(observer, arguments.seconds, generation)
        finally:
            resources = observer.close()
    assert_clean_shutdown(resources)
    return result | {"stream_resources": resources}


def measure_language(arguments, repository, language):
    with tempfile.TemporaryDirectory(prefix=f"ccn-idle-{language}-") as directory:
        project = Project(Path(directory), language, arguments.typescript_modules, repository)
        project.reset()
        session = Session(arguments.executable, project, arguments.timeout)
        baseline = session.analyze()
        artifact = project.root / ".codecallnavigator/graph.json"
        before = artifact.read_bytes()
        result = {"project_size": project_size(project), "modes": {}}
        for mode in arguments.modes:
            result["modes"][mode] = measure_mode(session, arguments, mode, baseline["generation"])
            if artifact.read_bytes() != before:
                raise RuntimeError("Idle monitor changed the canonical artifact")
            print(language, mode, result["modes"][mode]["mean_percent_one_core"], flush=True)
        return result


def execute(arguments):
    repository = Path(__file__).resolve().parents[1]
    arguments.executable = arguments.executable.resolve(strict=True)
    arguments.output.mkdir(parents=True, exist_ok=False)
    info = metadata(arguments.executable, repository) | {
        "hardware": hardware(),
        "analysis_resources": analysis_resources(arguments.executable, repository, arguments.typescript_modules),
        "purpose": "No saved edits; compiler baseline finishes before either monitor starts",
        "detection": "No edits or detection-latency measurement in these idle windows",
        "reset_protocol": "One independent compiler baseline per route, then sequential passive/automatic monitor processes without saved edits",
        "cpu": "Idle-window fields use CLI self CPU only; stream_resources totals use RUSAGE_CHILDREN and include sampler subprocesses and startup/shutdown",
        "stages": "Compiler baseline is outside the idle window; no compiler stage latency claim",
        "idle_monitor": "Single idle window per mode/route, not a statistical p95. CLI self CPU from ps cumulative centisecond time after initial/Watching readiness; CPU wall interval brackets endpoint ps calls and their overhead is reported; percentage of one core, not whole-machine CPU; sampler/helper CPU excluded",
        "sampling": "50ms process-tree RSS samples restricted to idle window; shared pages may be counted repeatedly; brief peaks can be missed",
        "command": ["python3.13", "tools/measure_freshness_idle.py", "--executable", str(arguments.executable),
                    "--output", str(arguments.output), "--seconds", str(arguments.seconds),
                    "--languages", ",".join(arguments.languages), "--modes", ",".join(arguments.modes),
                    "--typescript-modules", str(arguments.typescript_modules), "--timeout", str(arguments.timeout)]}
    (arguments.output / "metadata.json").write_text(json.dumps(info, indent=2) + "\n")
    results = {}
    for language in arguments.languages:
        try:
            results[language] = measure_language(arguments, repository, language)
        except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
            results[language] = {"failure": str(error)}
    result = {"schema": "codecallnavigator.freshness-idle-measurement.v1", "results": results}
    (arguments.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return int(any("failure" in value for value in results.values()))


def validate_arguments(arguments, parser):
    if arguments.seconds <= 0 or arguments.timeout <= 0:
        parser.error("seconds and timeout must be positive")
    if set(arguments.languages) - {"typescript", "csharp", "maven", "gradle"}:
        parser.error("unknown language/build route")
    if set(arguments.modes) - {"passive", "automatic"}:
        parser.error("unknown observation mode")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--typescript-modules", type=Path, required=True)
    parser.add_argument("--languages", type=lambda value: value.split(","),
                        default=["typescript", "csharp", "maven", "gradle"])
    parser.add_argument("--modes", type=lambda value: value.split(","), default=["passive", "automatic"])
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--timeout", type=float, default=120)
    arguments = parser.parse_args()
    validate_arguments(arguments, parser)
    raise SystemExit(execute(arguments))


if __name__ == "__main__":
    main()
