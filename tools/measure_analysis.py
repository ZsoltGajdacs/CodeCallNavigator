#!/usr/bin/env python3
"""Measure release analyses using isolated caches, including full graph equivalence.

Example: python3 tools/measure_analysis.py --executable target/release/codecallnavigator
  --workload storyengine=/path/backend --output /tmp/performance --runs 5
  --timings --baseline /tmp/baseline

Output includes macOS time(1) resource data and sampled temporary disk high-water
usage. Disk samples are lower bounds. They exclude Gradle/Maven caches and outputs.
"""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import threading
import time

from analysis_graph import compare, digest, graph_snapshot, volatile_generated_hashes


def disk_bytes(roots):
    paths = (Path(directory) / name for root in roots
             for directory, _, files in os.walk(root) for name in files)
    return sum(file_size(path) for path in paths)


def file_size(path):
    try:
        return path.lstat().st_size
    except FileNotFoundError:
        return 0


def process_tree_bytes(pid):
    output = subprocess.check_output(["/bin/ps", "-axo", "pid=,ppid=,rss="], text=True)
    processes = [tuple(map(int, line.split())) for line in output.splitlines()]
    selected, pending = set(), {pid}
    while pending:
        selected.update(pending)
        pending = {child for child, parent, _ in processes if parent in pending} - selected
    return sum(rss * 1024 for child, _, rss in processes if child in selected)


def sample_resources(stopped, context, samples):
    temporary = Path(tempfile.gettempdir())
    while not stopped.wait(0.25):
        roots = set(temporary.glob("codecallnavigator-*-*-*")) - context["before"]
        samples.append((disk_bytes(roots), process_tree_bytes(context["pid"])))


def run_analysis(arguments, name, target, number):
    directory = arguments.output / name / str(number)
    directory.mkdir(parents=True, exist_ok=False)
    cache = directory / "cache"
    command = ["/usr/bin/time", "-l", "-o", str(directory / "resources.txt"),
               str(arguments.executable), "analyze", target]
    if arguments.timings:
        command.extend(["--timings", str(directory / "timings.json")])
    stopped, samples = threading.Event(), []
    before = set(Path(tempfile.gettempdir()).glob("codecallnavigator-*-*-*"))
    started = time.monotonic()
    try:
        with (directory / "log.txt").open("w") as log:
            process = subprocess.Popen(command, env={**os.environ, "CODECALLNAVIGATOR_DATA_DIR": str(cache)},
                                       stdout=log, stderr=subprocess.STDOUT)
            sampler = threading.Thread(target=sample_resources,
                                       args=(stopped, {"before": before, "pid": process.pid}, samples))
            sampler.start()
            exit_code = process.wait()
        elapsed = time.monotonic() - started
    finally:
        stopped.set()
        if 'sampler' in locals():
            sampler.join()
    report = {"seconds": elapsed, "exit_code": exit_code,
              "sampled_temporary_bytes": max((disk for disk, _ in samples), default=0),
              "sampled_process_tree_rss_bytes": max((rss for _, rss in samples), default=0),
              "resources": (directory / "resources.txt").read_text()}
    enrich_report(arguments, directory, report)
    return report


def enrich_report(arguments, directory, report):
    cache = directory / "cache"
    timings = directory / "timings.json"
    if timings.exists():
        report["timings"] = json.loads(timings.read_text())
    if report["exit_code"]:
        report["failure"] = (directory / "log.txt").read_text()[-4000:]
        return
    snapshot = graph_snapshot(cache)
    report["graph"] = {"sha256": digest(snapshot), "nodes": len(snapshot["nodes"]),
                       "edges": len(snapshot["edges"])}
    reference = (arguments.baseline / directory.parent.name / "0" / "cache") if arguments.baseline else None
    if reference:
        name = directory.parent.name
        if name not in arguments.volatile_hashes:
            arguments.volatile_hashes[name] = volatile_generated_hashes((arguments.baseline / name).glob("*/cache"))
        report["equivalence"] = compare(reference, cache, arguments.volatile_hashes[name])


def summary(runs):
    successful = [run["seconds"] for run in runs if not run["exit_code"]]
    return {"runs": runs, "median_seconds": statistics.median(successful) if successful else None,
            "all_succeeded": len(successful) == len(runs),
            "all_equivalent": all(run.get("equivalence", {}).get("equal", True) for run in runs)}


def parse_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--workload", action="append", required=True, help="NAME=TARGET")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--timings", action="store_true")
    arguments = parser.parse_args()
    if arguments.runs < 1:
        parser.error("--runs must be positive")
    arguments.executable = arguments.executable.resolve(strict=True)
    arguments.output = arguments.output.resolve()
    arguments.volatile_hashes = {}
    return arguments


def measure_workload(arguments, workload):
    runs = []
    for number in range(arguments.runs):
        runs.append(run_analysis(arguments, *workload, number))
        print(f"{workload[0]} {number + 1}/{arguments.runs}: {runs[-1]['seconds']:.3f}s", flush=True)
    return summary(runs)


def workloads(values):
    result = {}
    for workload in values:
        name, target = workload.split("=", 1)
        valid = name and Path(name).name == name and name not in (".", "..")
        if not valid or name in result:
            raise ValueError("workload names must be distinct single path components")
        result[name] = target
    return result.items()


def main():
    arguments = parse_arguments()
    results = {}
    for workload in workloads(arguments.workload):
        results[workload[0]] = measure_workload(arguments, workload)
        (arguments.output / "summary.json").write_text(json.dumps(results, indent=2))
    return 0 if all(value["all_succeeded"] and value["all_equivalent"] for value in results.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
