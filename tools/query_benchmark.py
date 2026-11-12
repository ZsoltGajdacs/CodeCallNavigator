#!/usr/bin/env python3
"""Measure the public CLI boundary without changing the analyzed project."""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time


def checked_response(raw):
    response = json.loads(raw)
    if response.get("error"):
        raise RuntimeError(response["error"])
    return response


def summarize(samples):
    durations = [sample["milliseconds"] for sample in samples]
    return {"samples": samples, "median_ms": statistics.median(durations),
            "max_ms": max(durations)}


def measure(engine, project, repeats, include_render):
    queries = {"status": {"kind": "status"},
               "search": {"kind": "search", "text": "create", "limit": 50},
               "selection": {"kind": "selection", "selection": {"kind": "overview"}},
               "snapshot": {"kind": "snapshot"}}
    requests = {name: json.dumps({"version": 1, "project": str(project), "query": query}).encode()
                for name, query in queries.items()}
    if include_render:
        status=subprocess.run([str(engine),"query"],input=requests["status"],capture_output=True,check=True,timeout=120)
        generation=checked_response(status.stdout)["generation"]
        requests["render_selection"]=json.dumps({"version":1,"project":str(project),"expected_generation":generation,"query":{"kind":"render_selection","selection":{"kind":"overview"}}}).encode()
    return {"one_shot": measure_one_shot(engine, requests, repeats),
            "session": measure_session(engine, requests, repeats)}


def sample(raw, started):
    response = checked_response(raw)
    return {"milliseconds": round((time.perf_counter() - started) * 1000, 2),
            "response_bytes": len(raw), "generation": response["generation"]}


def measure_one_shot(engine, requests, repeats):
    results = {}
    for name, request in requests.items():
        samples = []
        for _ in range(repeats):
            started = time.perf_counter()
            output = subprocess.run([str(engine), "query"], input=request, capture_output=True,
                                    check=True, timeout=120)
            samples.append(sample(output.stdout, started))
        results[name] = summarize(samples)
    return results


def measure_session(engine, requests, repeats):
    with subprocess.Popen([str(engine), "query-session"], stdin=subprocess.PIPE,
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL) as session:
        try:
            return session_samples(session, requests, repeats)
        finally:
            session.stdin.close()
            session.wait(timeout=10)


def session_samples(session, requests, repeats):
    results = {}
    for name, request in requests.items():
        samples = []
        for _ in range(repeats):
            started = time.perf_counter()
            session.stdin.write(request + b"\n")
            session.stdin.flush()
            samples.append(sample(session.stdout.readline(), started))
        results[name] = summarize(samples)
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", type=Path, required=True)
    parser.add_argument("--project", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--render", action="store_true", help="Also measure the complete generation-bound desktop render selection")
    args = parser.parse_args()
    if args.repeats < 1:
        parser.error("--repeats must be positive")
    project = args.project.resolve()
    result = measure(args.engine.resolve(), project, args.repeats, args.render)
    result["artifact_bytes"] = (project / ".codecallnavigator/graph.json").stat().st_size
    result["method"] = "Wall time includes CLI execution, pipe transfer and client JSON decoding; first session status includes initial loading. Filesystem caches are uncontrolled."
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({mode: {name: value["median_ms"] for name, value in entries.items()}
                      for mode, entries in result.items() if mode in {"one_shot", "session"}}))


if __name__ == "__main__":
    main()
