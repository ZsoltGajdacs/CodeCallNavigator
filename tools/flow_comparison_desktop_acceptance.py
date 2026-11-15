#!/usr/bin/env python3
"""Run the opt-in comparison probe with compiler-produced saved C# revisions in a relocated app."""
import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import shutil
import sqlite3
import subprocess
import uuid

from csharp_flow_acceptance import require
from csharp_desktop_acceptance import (clean_environment, require_bundled_runtime,
                                       require_viewport, setup_window, write_json)
from comparison_desktop_fixture import prepare, git, HEAD_REF
from comparison_desktop_gate import Gate
from comparison_desktop_cleanup import OwnedRun
from comparison_desktop_protocol import NativeOperations
from native_fixture import NativeFixture


def seed_session(store, project):
    store.mkdir()
    session = {"root": str(project), "graph_context": {"project": str(project), "roots": [], "generation": None}}
    with sqlite3.connect(store / "catalog.sqlite3") as database:
        database.execute("CREATE TABLE catalog (key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        database.execute("INSERT INTO catalog VALUES ('session',?)", (json.dumps(session),))


def validate_report(report, app):
    require(report["version"] == 2 and report["status"] == "passed", f"Native comparison failed: {report}")
    require_viewport(report["viewport"])
    require(report["expected_viewport"] == [1050, 680], "Comparison probe used another minimum")
    require_bundled_runtime(report, app)
    checks = {"normal_no_graph_boot", "refs_entered_by_controls", "minimum_viewport_drawn",
              "owned_window_setup_acknowledged", "provenance_drawn", "mismatch_cleared_actionable_content",
              "explicit_reload_rebound", "cancel_waited_for_owned_query_cleanup",
              "fresh_request_after_cancel", "endpoint_target_followed", "back_pair_restored", "no_graph_returned"}
    checks |= {f"{role}_pair_exact" for role in ("Changed", "Unchanged", "Added", "Removed", "Legacy", "Manual")}
    checks |= {f"{side}_{check}" for side in ("Before", "After") for check in
               ("highlight_drawn", "camera_independent", "scroll_independent", "hidden_scroll_restored")}
    require(all(report["checks"].get(check) is True for check in checks), "Native comparison omitted a required drawn check")
    require(report["frames"]["drawn"] > 0 and report["ready_drawn_ms"], "Native comparison has no actual Drawn observations")
    require(report["captures"] == ["missing-source", "manual-pair", "restored-pair"], "Native comparison capture checkpoints are incomplete")


@dataclass(frozen=True)
class NativeRun:
    app: Path
    fixture: object
    store: Path
    directory: Path
    theme: str
    timeout: int


def run_native(run):
    app, fixture, store, directory, theme, timeout = run.app, run.fixture, run.store, run.directory, run.theme, run.timeout
    directory.mkdir()
    write_json(directory / "config.json", fixture.config)
    gate = Gate(fixture.project, directory, fixture.config["initial"]["base_commit"] +
                ":.codecallnavigator/graph.json", uuid.uuid4().hex, app / "Contents/MacOS/codecallnavigator-engine")
    environment = clean_environment()
    environment.update(CODECALLNAVIGATOR_DATA_DIR=str(store),
                       CODECALLNAVIGATOR_FLOW_ACCEPTANCE=str(directory), PATH=gate.install())
    command = [str(app / "Contents/MacOS/CodeCallNavigator"), "--native", "--trace",
               str(directory / "native.jsonl"), "--size", "1050x680", "--theme", theme]
    with (directory / "app.log").open("wb") as log:
        process = subprocess.Popen(command, env=environment, stdout=log, stderr=subprocess.STDOUT)
        lease = None
        try:
            lease = setup_window(process, directory, timeout)
            operations = NativeOperations(fixture, gate, process, directory, lease.identity.window_id, timeout).serve()
            require(process.wait(timeout=30) == 0, "Native comparison app returned failure")
            report = json.loads((directory / "report.json").read_text())
            validate_report(report, app)
            return {"theme": theme, "command": command, "observations": report,
                    "operations": operations, "window_setup": lease.audit}
        finally:
            OwnedRun(process, app / "Contents/MacOS/codecallnavigator-engine", gate, directory, lease).close()


def acceptance(app, output, timeout):
    output.mkdir(parents=True, exist_ok=True)
    require(not any((output / theme).exists() for theme in ("dark", "light")),
            "Use a new output directory; native evidence cannot be overwritten")
    with NativeFixture(output, 'flow-comparison-native-') as root:
        relocated = root / "Relocated CodeCallNavigator.app"
        shutil.copytree(app, relocated, symlinks=True)
        fixture = prepare(relocated / "Contents/MacOS/codecallnavigator-engine", root / "project", timeout)
        results = []
        for theme in ("dark", "light"):
            # Each run starts from the exact original immutable endpoint binding.
            git(fixture.project, "update-ref", HEAD_REF, fixture.config["initial"]["head_commit"])
            store = root / f"store-{theme}"
            seed_session(store, fixture.project)
            results.append(run_native(NativeRun(relocated, fixture, store, output / theme, theme, timeout)))
        return {"version": 2, "status": "passed", "metrics": fixture.metrics, "native": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=180)
    args = parser.parse_args()
    require(30 <= args.timeout <= 300, "Timeout must be 30..300 seconds")
    output = args.output.resolve()
    try:
        report = acceptance(args.app.resolve(), output, args.timeout)
    except Exception as error:
        output.mkdir(parents=True, exist_ok=True)
        write_json(output / "result.json", {"version": 2, "status": "failed", "error": str(error)})
        raise
    write_json(output / "result.json", report)
    print(f"Native comparison acceptance passed: {output}")


if __name__ == "__main__":
    main()
