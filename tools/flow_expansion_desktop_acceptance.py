#!/usr/bin/env python3
"""Verify native expansion controls against real packaged C# analysis.

Only the assembled native window can prove Drawn expansion/collapse, occurrence
selection, source-owner switching and Back camera/scroll restoration. Real CLI
preflight proves the multi-file snapshots survive removal of live source; unit
and integration tests retain budget, stale-generation and malformed-reply cases.
This helper never fabricates a successful analyzer or expands a modeled chart.
"""
import argparse
from dataclasses import dataclass
import hashlib
import json
import math
from pathlib import Path
import shutil
import subprocess
import time

from csharp_flow_acceptance import require
from csharp_desktop_acceptance import (VIEWPORT, capture_window, clean_environment, defer_for_desktop,
                                       require_bundled_runtime,
                                       require_viewport, seed_session, setup_window,
                                       wait_request, write_json)
from comparison_desktop_cleanup import OwnedRun
from flow_expansion_desktop_fixture import prepare
from native_fixture import NativeFixture

CHECKS = {"root_drawn", "two_distinct_call_slots", "first_child_expanded",
          "second_child_expanded", "source_owner_switched", "recursive_cycle_stop_visible",
          "collapse_retains_other_child", "callee_followed",
          "back_restores_expansion_camera_scroll", "collapse_all_restores_ordinary",
          "generation_unchanged", "source_snapshot_exact", "owned_window_setup_acknowledged"}


@dataclass(frozen=True)
class NativeRun:
    app: Path
    fixture: object
    store: Path
    directory: Path
    theme: str
    timeout: int


def validate_report(report, run, identity):
    require(report["version"] == 3 and report["status"] == "passed",
            f"Native expansion failed: {report}")
    require(report["generation"] == run.fixture.generation
            and report["root_symbol"] == run.fixture.entry,
            "Native expansion report has another root or generation")
    require(report["process_id"] == identity.process_id and report["window_id"] == identity.window_id,
            "Native expansion report belongs to another process or window")
    require(report["captures"] == ["restored-expanded-root"], "Expansion capture checkpoint is incomplete")
    require_viewport(report["viewport"])
    require(report["expected_viewport"] == list(VIEWPORT), "Expansion probe used another minimum")
    require_bundled_runtime(report, run.app)
    require(all(report["checks"].get(check) is True for check in CHECKS),
            "Native expansion omitted a required observable control check")
    require(report["frames"]["drawn"] > 0, "Native expansion has no Drawn observation")
    validate_measurements(report["ready_drawn"])


def validate_measurements(samples):
    operations = {"flow-expand-mode", "expand-call", "flow-collapse-occurrence", "flow-collapse-all"}
    require(len(samples) >= 6 and operations <= {sample["operation"] for sample in samples},
            "Native expansion omitted a first-Drawn readiness measurement")
    for sample in samples:
        require(nonnegative(sample["milliseconds"]) and min(sample["steps"], sample["links"]) > 0,
                "Invalid native readiness or chart-size measurement")
        if sample["operation"] != "flow-collapse-all":
            require(all(nonnegative(sample[key]) for key in ("query_milliseconds", "layout_milliseconds")),
                    "Missing cold-query or Graphviz layout measurement")


def nonnegative(value):
    return isinstance(value, (int, float)) and math.isfinite(value) and value >= 0


def capture_checkpoint(process, run, identity):
    request = wait_request(process, run.directory, "capture-request.json",
                           time.monotonic() + run.timeout)
    try:
        require(request["window_id"] == identity.window_id,
                "Expansion capture escaped the owned native window")
        require(request["checkpoint"] == "restored-expanded-root",
                "Expansion requested another capture checkpoint")
        capture = capture_window(request, run.directory, run.fixture)
    except Exception as error:
        write_json(run.directory / "capture-ack.json",
                   {"version": 1, "success": False, "error": str(error)})
        raise
    write_json(run.directory / "capture-ack.json", {"version": 1, "success": True})
    return capture


def run_native(run):
    directory = run.directory
    directory.mkdir()
    write_json(directory / "config.json", run.fixture.config)
    environment = clean_environment()
    environment.update(CODECALLNAVIGATOR_DATA_DIR=str(run.store),
                       CODECALLNAVIGATOR_FLOW_ACCEPTANCE=str(directory), PATH="/usr/bin:/bin")
    command = [str(run.app / "Contents/MacOS/CodeCallNavigator"), "--native", "--trace",
               str(directory / "native.jsonl"), "--size", "1050x680", "--theme", run.theme]
    with (directory / "app.log").open("wb") as log:
        process = subprocess.Popen(command, env=environment, stdout=log, stderr=subprocess.STDOUT)
        lease = None
        try:
            lease = setup_window(process, directory, run.timeout)
            capture = capture_checkpoint(process, run, lease.identity)
            require(process.wait(timeout=30) == 0, "Native expansion app returned failure")
            report = json.loads((directory / "report.json").read_text())
            validate_report(report, run, lease.identity)
            return {"theme": run.theme, "command": command, "capture": capture,
                    "observations": report, "window_setup": lease.audit}
        finally:
            OwnedRun(process, run.app / "Contents/MacOS/codecallnavigator-engine",
                     None, directory, lease).close()


def acceptance(app, output, timeout):
    output.mkdir(parents=True, exist_ok=True)
    require(not any((output / theme).exists() for theme in ("dark", "light")),
            "Use a new output directory; native expansion evidence cannot be overwritten")
    with NativeFixture(output, "flow-expansion-native-") as root:
        relocated = root / "Relocated CodeCallNavigator.app"
        shutil.copytree(app, relocated, symlinks=True)
        fixture = prepare(relocated / "Contents/MacOS/codecallnavigator-engine", root / "project", timeout)
        results = []
        for theme in ("dark", "light"):
            store = root / f"store-{theme}"
            seed_session(store, fixture)
            results.append(run_native(NativeRun(relocated, fixture, store, output / theme, theme, timeout)))
        artifact = fixture.project / ".codecallnavigator/graph.json"
        require(hashlib.sha256(artifact.read_bytes()).hexdigest() == fixture.metrics["artifact_sha256"],
                "Native expansion mutated the analyzed artifact")
        fixture.metrics["no_mutated_artifact"] = True
        return {"version": 3, "status": "passed", "metrics": fixture.metrics, "native": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()
    require(30 <= args.timeout <= 300, "Timeout must be 30..300 seconds")
    output = args.output.resolve()
    if status := defer_for_desktop(output, version=3):
        return status
    try:
        report = acceptance(args.app.resolve(), output, args.timeout)
    except Exception as error:
        output.mkdir(parents=True, exist_ok=True)
        write_json(output / "result.json", {"version": 3, "status": "failed", "error": str(error)})
        raise
    write_json(output / "result.json", report)
    print(f"Native flow expansion acceptance passed: {output}")


if __name__ == "__main__":
    raise SystemExit(main())
