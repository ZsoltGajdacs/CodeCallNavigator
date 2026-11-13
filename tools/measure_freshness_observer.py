#!/usr/bin/env python3
"""Measure save-to-verified-emission through the production CLI observe stream.

Compiler projects and dependency caches are disposable/shared respectively, as
in the original feasibility harness. Each scenario has one baseline analysis;
samples restore its inputs and repeat the same edit without rebuilding. The
first sample is separate from at least twenty requested warm repetitions.
"""
import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import tempfile
import time

from freshness_measurement import distribution, response, run
from freshness_observer_measurement import Boundary, Observer, observe_once, request
from freshness_scenarios import Project, SCENARIOS
from measure_freshness import Session, metadata


@dataclass
class RevisionBinding:
    baseline: dict
    edited: str
    sequence: int


def required_revision(observation):
    if not observation.get("input_revision"):
        raise RuntimeError(f"Workload has no usable observed-input revision: {observation}")
    return observation["input_revision"]


def revision_binding(baseline, edited, sequence):
    before, after = required_revision(baseline), required_revision(edited)
    if before == after:
        raise RuntimeError("Saved edit did not change the observed-input revision")
    return RevisionBinding(baseline, after, sequence)


def sample(observer, session, scenario, binding):
    baseline = binding.baseline
    started = time.monotonic()
    session.project.edit(scenario, 0)
    saved = time.monotonic()
    received, event = observer.wait(lambda item: item["input_revision"] == binding.edited,
                                    Boundary(binding.sequence, started))
    observation = event["observation"]
    if observation["input_status"] != "stale" or observation["generation"] != baseline["generation"]:
        raise RuntimeError(f"Edit did not produce generation-bound stale status: {event}")
    record = {"save_to_verified_ms": (received - started) * 1000,
              "save_completion_to_verified_ms": (received - saved) * 1000,
              "save_ms": (saved - started) * 1000, "baseline_sequence": binding.sequence, "event": event}
    reset = time.monotonic()
    session.project.reset(scenario)
    _, restored = observer.wait(lambda item: item["input_revision"] == baseline["input_revision"],
                                Boundary(event["sequence"], reset))
    binding.sequence = restored["sequence"]
    return record


def assert_clean_shutdown(resources):
    expected = {"exit_code": 0, "timed_out": False, "reader_stopped": True, "output_eof": True}
    failures = [field for field, wanted in expected.items() if resources[field] != wanted]
    if failures:
        raise RuntimeError(f"Observer shutdown failed {failures}: {resources}")


def project_size(project):
    raw = (project.root / ".codecallnavigator/graph.json").read_bytes()
    graph = json.loads(raw)["graph"]
    return {"artifact_bytes": len(raw), "nodes": len(graph["nodes"]), "relationships": len(graph["edges"]),
            "observed_inputs": len(graph["analysis"]["inputs"]),
            "authored_source_files": sum(path.suffix in {".ts", ".java", ".cs"} for path in project.source.parent.iterdir())}


def scenario(session, name, arguments, output):
    session.project.reset(name)
    analysis = session.analyze()
    size = project_size(session.project)
    baseline = observe_once(session.executable, session.project)
    if baseline["input_status"] == "stale":
        raise RuntimeError(f"Newly published baseline was stale: {baseline}")
    session.project.edit(name, 0)
    edited = observe_once(session.executable, session.project)
    session.project.reset(name)
    observer = Observer([str(session.executable), "observe"], request(session.project), arguments.timeout)
    records = []
    try:
        _, initial = observer.wait(lambda item: item["input_revision"] == baseline["input_revision"])
        binding = revision_binding(baseline, edited, initial["sequence"])
        for iteration in range(arguments.runs + 1):
            record = sample(observer, session, name, binding)
            record.update(language=session.project.language, scenario=name,
                          phase="first" if iteration == 0 else "warm", iteration=iteration)
            records.append(record)
            output.write(json.dumps(record) + "\n")
            output.flush()
    finally:
        resources = observer.close()
    assert_clean_shutdown(resources)
    warm = [item["save_to_verified_ms"] for item in records if item["phase"] == "warm"]
    return {"project_size": size, "baseline_analysis": analysis, "baseline_observation": baseline, "edited_reference": edited,
            "save_to_verified_ms": distribution(warm), "one_second_p95": distribution(warm)["p95"] <= 1000,
            "stream_resources": resources}


def reproduction_command(arguments, executable):
    command = ["python3.13", "tools/measure_freshness_observer.py", "--executable", str(executable),
               "--output", str(arguments.output), "--runs", str(arguments.runs),
               "--languages", ",".join(arguments.languages), "--scenarios", ",".join(arguments.scenarios),
               "--timeout", str(arguments.timeout)]
    if arguments.typescript_modules is not None:
        command.extend(["--typescript-modules", str(arguments.typescript_modules.resolve())])
    return command


def hardware():
    try:
        return run(["sysctl", "machdep.cpu.brand_string", "hw.memsize", "hw.physicalcpu", "hw.logicalcpu"], timeout=30)
    except OSError as error:
        return {"unavailable": str(error)}


def execute(arguments):
    repository = Path(__file__).resolve().parents[1]
    executable = arguments.executable.resolve(strict=True)
    arguments.output.mkdir(parents=True, exist_ok=False)
    capabilities = response(run([str(executable), "capabilities"]))
    info = metadata(executable, repository) | {
        "capabilities": capabilities, "hardware": hardware(),
        "source_state": "Measured from the working tree, including local changes; source_commit records HEAD, executable_sha256 identifies the measured binary",
        "detection": "save start to receipt of complete verified NDJSON line; includes write duration; final revision established independently through one-shot freshness before stream startup",
        "idle_monitor": "stream resource totals include initialization, edits and 50ms sampler overhead; no isolated steady-state CPU claim",
        "reset_protocol": "one real baseline analysis per scenario; restore exact saved inputs and await baseline revision, then repeat identical edit; no refresh between repetitions",
        "command": reproduction_command(arguments, executable)}
    (arguments.output / "metadata.json").write_text(json.dumps(info, indent=2) + "\n")
    results = {}
    with (arguments.output / "samples.jsonl").open("w") as output:
        for language in arguments.languages:
            with tempfile.TemporaryDirectory(prefix=f"ccn-observer-{language}-") as directory:
                try:
                    project = Project(Path(directory), language, arguments.typescript_modules, repository)
                    project.reset()
                    session = Session(executable, project, arguments.timeout)
                    for name in arguments.scenarios:
                        key = f"{language}/{name}"
                        results[key] = scenario(session, name, arguments, output)
                        print(key, results[key]["save_to_verified_ms"], flush=True)
                except (OSError, RuntimeError, ValueError) as error:
                    results[f"{language}/unavailable"] = {"failure": str(error)}
    result = {"schema": "codecallnavigator.freshness-observer-measurement.v1",
              "requested_warm_runs": arguments.runs, "results": results}
    (arguments.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return int(any("failure" in item for item in results.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--typescript-modules", type=Path)
    parser.add_argument("--languages", type=lambda value: value.split(","), default=["typescript", "csharp", "maven", "gradle"])
    parser.add_argument("--scenarios", type=lambda value: value.split(","), default=list(SCENARIOS))
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=120)
    arguments = parser.parse_args()
    if arguments.runs < 1:
        parser.error("runs must be positive")
    if arguments.timeout <= 0:
        parser.error("timeout must be positive")
    if set(arguments.scenarios) - set(SCENARIOS):
        parser.error("scenarios must be documented names")
    raise SystemExit(execute(arguments))


if __name__ == "__main__":
    main()
