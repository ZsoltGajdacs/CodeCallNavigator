#!/usr/bin/env python3
"""Measure one saved Java body edit in an explicitly disposable corpus copy.

Requires --project beneath the system temporary directory. Never copies or edits
the original checkout. Full semantic equivalence requires a separate corpus
oracle; this probe reports that unavailable rather than using counts as proof.
"""
import argparse
import json
from pathlib import Path
import tempfile
import time

from freshness_measurement import distribution, response, run


def analyze(executable, project, targets):
    measurement = run([str(executable), "serve-once"], {
        "command": "analyze", "version": 1, "project": str(project),
        "roots": [], "targets": targets, "infer_linked_roots": False,
    }, timeout=180, keep_open=True)
    events = [json.loads(line) for line in measurement["stdout"].splitlines()]
    complete = next((event for event in events if event["event"] == "complete"), None)
    if measurement["exit_code"] or complete is None:
        raise RuntimeError(measurement["stderr"][-2000:] or str(events)[-2000:])
    measurement["metrics"] = complete["metrics"]
    measurement["generation"] = complete["digest"]
    return measurement


def status(executable, project, generation=None):
    request = {"version": 1, "project": str(project), "roots": [], "query": {"kind": "status"}}
    if generation:
        request["expected_generation"] = generation
    measurement = run([str(executable), "query"], request)
    return measurement, response(measurement)


def sample(executable, project, targets, baseline):
    source, artifact, pristine, before = baseline
    artifact.write_bytes(before)
    source.write_text(pristine)
    started = time.monotonic()
    source.write_text(pristine.replace("return 1;", "return 2;"))
    detection, stale = status(executable, project)
    if stale["freshness"]["input_status"] != "stale":
        raise RuntimeError("Saved corpus edit was not detected")
    refresh = analyze(executable, project, targets)
    adoption, observed = status(executable, project, refresh["generation"])
    if observed["error"] or observed["generation"] != refresh["generation"]:
        raise RuntimeError("Corpus generation adoption failed")
    return {"detection": detection, "refresh": refresh, "adoption": adoption,
            "save_to_adoption_ms": (time.monotonic() - started) * 1000,
            "freshness_after": observed["freshness"], "nodes": observed["result"]["nodes"],
            "edges": observed["result"]["edges"],
            "correctness": "unavailable: no independent clean large-corpus semantic oracle"}


def temporary_project(project):
    roots = [Path(tempfile.gettempdir()).resolve(), Path('/tmp').resolve()]
    return any(project.is_relative_to(root) and project != root for root in roots)


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=20)
    args = parser.parse_args()
    project = args.project.resolve(strict=True)
    if not temporary_project(project) or args.runs < 1:
        parser.error("project must be an owned temporary copy; runs must be positive")
    args.project = project
    return args


def summarize(records):
    warm = records[1:]
    summary = {"save_to_adoption_ms": distribution([record["save_to_adoption_ms"] for record in warm])}
    for operation in ["detection", "refresh", "adoption"]:
        summary[operation + "_ms"] = distribution([record[operation]["milliseconds"] for record in warm])
    summary["method"] = "685-file StoryEngine copy plus one probe; identical body edits; baseline artifact restored; dependency/build caches retained; source generation may rewrite other observed inputs"
    summary["correctness"] = "unavailable: independent clean large-corpus oracle"
    return summary


def prepare_corpus(executable, project):
    """Create the owned probe and capture one full-analysis baseline."""
    source = project / "backend/src/main/java/freshnessprobe/FreshnessCorpusProbe.java"
    if source.exists():
        raise ValueError("probe source already exists; use a fresh disposable copy")
    source.parent.mkdir(parents=True, exist_ok=True)
    pristine = "package freshnessprobe; public class FreshnessCorpusProbe { public static int leaf(){return 1;} public static int start(){return leaf();} }\n"
    source.write_text(pristine)
    targets = response(run([str(executable), "discover", str(project / "backend")]))["targets"]
    analyze(executable, project, targets)
    artifact = project / ".codecallnavigator/graph.json"
    return targets, (source, artifact, pristine, artifact.read_bytes())


def main():
    args = arguments()
    project = args.project
    executable = args.executable.resolve(strict=True)
    args.output.mkdir(parents=True, exist_ok=False)
    targets, baseline = prepare_corpus(executable, project)
    records = []
    with (args.output / "samples.jsonl").open("w") as output:
        for iteration in range(args.runs + 1):
            record = sample(executable, project, targets, baseline)
            record.update(iteration=iteration, phase="cold" if iteration == 0 else "warm")
            records.append(record)
            output.write(json.dumps(record) + "\n")
            output.flush()
            print(f"corpus/body {iteration}/{args.runs}: ok", flush=True)
    (args.output / "summary.json").write_text(json.dumps(summarize(records), indent=2) + "\n")


if __name__ == "__main__":
    main()
