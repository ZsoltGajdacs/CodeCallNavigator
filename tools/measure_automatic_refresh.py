#!/usr/bin/env python3
"""Measure actual automatic publications against two independent clean analyses.

Each disposable project alternates two saved body variants. The first automatic
publication is separate from twenty warm publications. Every resulting artifact
must byte-match an independently analyzed, clean project with the same variant.
"""
import argparse
from dataclasses import dataclass
import gzip
import hashlib
import json
import os
from pathlib import Path
import tempfile
import time

from freshness_measurement import distribution, response, run
from freshness_observer_measurement import Observer, request
from freshness_scenarios import Project
from measure_freshness import Session, metadata
from measure_freshness_observer import assert_clean_shutdown, hardware, project_size


@dataclass
class PublicationBinding:
    references: dict
    generations: dict
    sequence: int = 0


def write_variant(project, variant):
    project.source.write_text(project.text(variant + 1))


def independent_references(session, output):
    references, measurements = {}, {}
    for variant in (0, 1):
        with tempfile.TemporaryDirectory(prefix="ccn-auto-reference-") as directory:
            # Gradle identities intentionally retain the checkout basename.
            root = Path(directory)
            if session.project.language == "gradle":
                root = root / session.project.root.name
            project = Project(root, session.project.language, session.project.modules,
                              session.project.repository)
            project.reset()
            write_variant(project, variant)
            measurements[str(variant)] = Session(session.executable, project, session.timeout).analyze()
            raw = (project.root / ".codecallnavigator/graph.json").read_bytes()
            references[variant] = raw
            path = output / f"{project.language}-variant-{variant}.json.gz"
            path.write_bytes(gzip.compress(raw, mtime=0))
    if references[0] == references[1]:
        raise RuntimeError("Body variants produced identical artifacts")
    return references, measurements


def observed_generation(event, generation):
    observation = event.get("observation")
    if observation is None or observation.get("generation") != generation:
        return False
    if observation["input_status"] == "stale":
        raise RuntimeError(f"Published generation remained stale: {event}")
    return True


def check_activity(event):
    activity = event.get("activity", {})
    if activity.get("state") in ("failed", "unavailable", "cancelled"):
        raise RuntimeError(f"Automatic refresh did not publish: {event}")


def wait_publication(observer, generation, started, after_sequence=0):
    deadline, events = time.monotonic() + observer.timeout, []
    while True:
        received, event = observer.next_event(deadline)
        if received < started or event.get("sequence", 0) <= after_sequence:
            continue
        events.append({"received_after_save_ms": (received - started) * 1000, "event": event})
        check_activity(event)
        if observed_generation(event, generation):
            return received, events


def completion_activity(event, generation):
    activity = event.get("activity", {})
    if activity.get("state") == "published" and activity.get("generation") == generation:
        return activity
    return None


def recorded_completion(events, generation):
    for entry in events:
        if completed := completion_activity(entry["event"], generation):
            return completed
    return None


def collect_completion(observer, generation, started, events):
    # Hold the saved bytes stable until the current outcome is delivered. This
    # captures compiler metrics without treating a coalesced stream as a journal.
    if completed := recorded_completion(events, generation):
        return completed
    deadline = time.monotonic() + observer.timeout
    while True:
        received, event = observer.next_event(deadline)
        check_activity(event)
        events.append({"received_after_save_ms": (received - started) * 1000, "event": event})
        if completed := completion_activity(event, generation):
            return completed


def sample(session, observer, binding, iteration):
    variant = (iteration + 1) % 2
    started = time.monotonic()
    write_variant(session.project, variant)
    saved = time.monotonic()
    received, events = wait_publication(observer, binding.generations[variant], started, binding.sequence)
    completion = collect_completion(observer, binding.generations[variant], started, events)
    if completion["runs_started"] != iteration + 1:
        raise RuntimeError(f"Unexpected automatic analysis count: {completion}")
    raw = (session.project.root / ".codecallnavigator/graph.json").read_bytes()
    if raw != binding.references[variant]:
        mismatch = Path(tempfile.mkdtemp(prefix="ccn-auto-mismatch-"))
        (mismatch / "actual.json").write_bytes(raw)
        (mismatch / "expected.json").write_bytes(binding.references[variant])
        raise RuntimeError(f"Automatic publication differs from independent analysis: {mismatch}")
    binding.sequence = events[-1]["event"]["sequence"]
    stale_times = [entry["received_after_save_ms"] for entry in events
                   if entry["event"].get("observation", {}).get("input_status") == "stale"]
    return {"language": session.project.language, "scenario": "alternating_body",
            "iteration": iteration, "phase": "first" if iteration == 0 else "warm",
            "variant": variant, "generation": binding.generations[variant], "equivalent": True,
            "reference_sha256": hashlib.sha256(raw).hexdigest(),
            "save_ms": (saved - started) * 1000,
            "save_to_verified_publication_ms": (received - started) * 1000,
            "save_completion_to_verified_publication_ms": (received - saved) * 1000,
            "first_stale_emission_ms": min(stale_times) if stale_times else None,
            "completion_activity": completion,
            "events": events}


def trace_event(output, received, event):
    output.write(json.dumps({"received_monotonic_seconds": received, "event": event}) + "\n")
    output.flush()


def measure_language(session, arguments, output):
    references, reference_measurements = independent_references(session, arguments.output)
    baseline = session.analyze()
    artifact = session.project.root / ".codecallnavigator/graph.json"
    if artifact.read_bytes() != references[0]:
        raise RuntimeError("Initial artifact differs from independent baseline analysis")
    generations = {variant: reference_measurements[str(variant)]["generation"] for variant in (0, 1)}
    records = []
    trace_path = arguments.output / f"{session.project.language}-stream.jsonl"
    with trace_path.open("w") as trace:
        observer = Observer([str(session.executable), "observe", "--auto-refresh"],
                            request(session.project), arguments.timeout,
                            on_event=lambda received, event: trace_event(trace, received, event))
        try:
            _, initial = observer.wait(lambda observation: observation["generation"] == baseline["generation"])
            binding = PublicationBinding(references, generations, initial["sequence"])
            for iteration in range(arguments.runs + 1):
                record = sample(session, observer, binding, iteration)
                records.append(record)
                output.write(json.dumps(record) + "\n")
                output.flush()
                print(f'{session.project.language} {iteration}/{arguments.runs}: '
                      f'{record["save_to_verified_publication_ms"]:.1f}ms', flush=True)
        finally:
            resources = observer.close()
    assert_clean_shutdown(resources)
    warm = [record["save_to_verified_publication_ms"] for record in records if record["phase"] == "warm"]
    return {"project_size": project_size(session.project), "baseline_analysis": baseline,
            "independent_analyses": reference_measurements, "stream_resources": resources,
            "warm_save_to_verified_publication_ms": distribution(warm),
            "first_save_to_verified_publication_ms": records[0]["save_to_verified_publication_ms"],
            "complete_artifact_comparisons": len(records)}


def reproduction_command(arguments, executable):
    command = ["python3.13", "tools/measure_automatic_refresh.py", "--executable", str(executable),
               "--output", str(arguments.output), "--runs", str(arguments.runs),
               "--languages", ",".join(arguments.languages), "--timeout", str(arguments.timeout)]
    if arguments.typescript_modules is not None:
        command.extend(["--typescript-modules", str(arguments.typescript_modules.resolve())])
    return command


def analysis_resources(executable, repository, modules):
    bundled = executable.parent.parent / "Resources"
    default = bundled if (bundled / "analyzers").is_dir() else repository
    root = Path(os.environ.get("CODECALLNAVIGATOR_RESOURCES", default))
    candidates = {
        "callatlas": ["analyzers/callatlas/callatlas.dll", "vendor/callatlas/src/CallAtlas.Cli/bin/Release/net11.0/callatlas.dll"],
        "bytecode": ["analyzers/java-bytecode/lib/codecallnavigator-bytecode.jar", "tools/java-bytecode/build/install/codecallnavigator-bytecode/lib/codecallnavigator-bytecode.jar"],
        "maven_metadata": ["analyzers/maven-metadata.jar", "tools/maven-metadata/build/libs/codecallnavigator-maven-metadata.jar"],
        "gradle_metadata": ["tools/gradle-metadata.gradle", "analyzers/gradle-metadata.gradle"],
        "typescript": ["analyzers/typescript/analyze.js", "tools/typescript/analyze.js"]}
    result = {}
    for name, paths in candidates.items():
        found = next((root / path for path in paths if (root / path).is_file()), None)
        result[name] = resource_fingerprint(found)
    result["gradle_wrapper"] = resource_fingerprint(repository / "fixtures/gradle/gradle/wrapper/gradle-wrapper.properties")
    if modules is not None:
        result["typescript_version"] = json.loads((modules / "typescript/package.json").read_text())["version"]
    return {"root": str(root.resolve()), "files": result}


def resource_fingerprint(path):
    if path is None or not path.is_file():
        return {"unavailable": "Resource is not installed"}
    return {"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def execute(arguments):
    repository = Path(__file__).resolve().parents[1]
    executable = arguments.executable.resolve(strict=True)
    arguments.output.mkdir(parents=True, exist_ok=False)
    capabilities = response(run([str(executable), "capabilities"]))
    info = metadata(executable, repository) | {
        "capabilities": capabilities, "hardware": hardware(),
        "analysis_resources": analysis_resources(executable, repository, arguments.typescript_modules),
        "command": reproduction_command(arguments, executable),
        "source_state": "Working tree; binary SHA identifies measured executable; source_commit identifies prior HEAD",
        "detection": "save start through receipt of verified observation of the expected new generation",
        "reset_protocol": "two independently fully analyzed clean references; alternating saved body bytes; no manual analysis between automatic publications",
        "stages": "save-to-publication timing ends at verified new-generation receipt; saved bytes then held stable until matching current Published activity delivers compiler metrics; coalesced stream is not a durable stage journal; separate atomic rename timestamp remains unavailable",
        "idle_monitor": "stream process-tree RSS and CPU totals cover initialization and all publications; no isolated steady-state CPU claim",
        "unavailable": ["desktop frame adoption latency", "large-project publication distributions",
                        "cold compiler/dependency caches", "separate atomic rename timestamp"]}
    (arguments.output / "metadata.json").write_text(json.dumps(info, indent=2) + "\n")
    results = {}
    with (arguments.output / "samples.jsonl").open("w") as output:
        for language in arguments.languages:
            try:
                with tempfile.TemporaryDirectory(prefix=f"ccn-auto-{language}-") as directory:
                    project = Project(Path(directory), language, arguments.typescript_modules, repository)
                    project.reset()
                    session = Session(executable, project, arguments.timeout)
                    results[language] = measure_language(session, arguments, output)
            except (OSError, RuntimeError, ValueError) as error:
                results[language] = {"failure": str(error)}
                print(f"{language}: {error}", flush=True)
    result = {"schema": "codecallnavigator.automatic-refresh-measurement.v1",
              "requested_warm_runs": arguments.runs, "results": results}
    (arguments.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    return int(any("failure" in item for item in results.values()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--typescript-modules", type=Path)
    parser.add_argument("--languages", type=lambda value: value.split(","),
                        default=["typescript", "csharp", "maven", "gradle"])
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=180)
    arguments = parser.parse_args()
    if arguments.runs < 1 or arguments.timeout <= 0:
        parser.error("runs and timeout must be positive")
    if set(arguments.languages) - {"typescript", "csharp", "maven", "gradle"}:
        parser.error("unsupported language/build route")
    raise SystemExit(execute(arguments))


if __name__ == "__main__":
    main()
