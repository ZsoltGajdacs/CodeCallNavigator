#!/usr/bin/env python3
"""Exercise a relocated native app using an isolated, compiler-analyzed C# fixture."""
import argparse
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import struct
import subprocess
import tempfile
import time

from csharp_flow_acceptance import require
from csharp_flow_ingestion_acceptance import Client
from native_window_setup import WindowIdentity, WindowLease


ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/flow/csharp-desktop"
VIEWPORT = (1050, 680)


@dataclass(frozen=True)
class Analyzed:
    project: Path
    entry: str
    target: str
    source: str
    generation: str
    metrics: dict


@dataclass(frozen=True)
class NativeRun:
    app: Path
    fixture: Analyzed
    store: Path
    directory: Path
    theme: str
    timeout: int


def write_json(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def clean_environment():
    environment = dict(os.environ)
    for key in list(environment):
        if key.startswith("CODECALLNAVIGATOR_"):
            environment.pop(key)
    return environment


def callable_node(snapshot, label, parameter):
    matches = [node for node in snapshot["result"]["nodes"]
               if node["label"] == label and parameter in node["signature"]]
    require(len(matches) == 1, f"Expected one {label}({parameter}) callable")
    return matches[0]["id"]


def prepare_fixture(engine, project):
    shutil.copytree(FIXTURE, project)
    projects = list(project.glob("*.csproj"))
    require(len(projects) == 1, "Desktop fixture must have one project")
    client = Client(engine, clean_environment())
    started = time.monotonic()
    client.analyze(project, projects[0])
    refresh_seconds = time.monotonic() - started
    snapshot = client.query(project, {"kind": "snapshot"})
    entry = callable_node(snapshot, "Run", "System.Boolean")
    target = callable_node(snapshot, "Leaf", "System.Int32")
    started = time.monotonic()
    flow = client.query(project, {"kind": "flow", "symbol": entry}, snapshot["generation"])
    query_seconds = time.monotonic() - started
    source = flow["result"]["source"]["content"]
    require(source.encode("utf-8") == (project / "Example.cs").read_bytes(),
            "CLI flow must preserve fixture BOM, Unicode and CRLF bytes")
    artifact_path = project / ".codecallnavigator/graph.json"
    artifact = json.loads(artifact_path.read_bytes())
    inventory = artifact["graph"]["flow_inventory"]
    metrics = {
        "fixture": "fixtures/flow/csharp-desktop",
        "refresh_seconds": refresh_seconds,
        "direct_flow_query_seconds": query_seconds,
        "artifact_bytes": artifact_path.stat().st_size,
        "inventory_compact_json_bytes": len(json.dumps(inventory, separators=(",", ":")).encode()),
        "source_bytes": len(source.encode("utf-8")),
        "artifact_sha256": hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
        "refresh_overhead": "No comparable pre-flow timing baseline; refresh_seconds is total elapsed time.",
        "commands": client.records,
    }
    return Analyzed(project, entry, target, source, snapshot["generation"], metrics)


def seed_session(directory, fixture):
    directory.mkdir()
    session = {
        "root": str(fixture.project),
        "graph_context": {"project": str(fixture.project), "roots": [], "generation": None},
        "navigation": {"grouping": "Method", "selected": fixture.entry,
                       "focus": fixture.entry, "focus_target": {"Method": fixture.entry}},
    }
    with sqlite3.connect(directory / "catalog.sqlite3") as connection:
        connection.execute("CREATE TABLE catalog (key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        connection.execute("INSERT INTO catalog VALUES ('session',?)", (json.dumps(session),))


def wait_request(process, directory, name, deadline):
    request = directory / name
    while time.monotonic() < deadline:
        if request.exists():
            return json.loads(request.read_text())
        require(process.poll() is None, f"Native app exited before {name}; see {directory / 'app.log'}")
        time.sleep(0.05)
    raise RuntimeError(f"Native app did not reach {name} before timeout")


def setup_window(process, directory, timeout):
    request = wait_request(process, directory, "window-request.json", time.monotonic() + timeout)
    require(request["version"] == 1 and request["process_id"] == process.pid,
            "Window setup belongs to another app process")
    require(tuple(request["expected_viewport"]) == VIEWPORT, "Window setup requested another viewport")
    lease = WindowLease(WindowIdentity(process.pid, request["window_id"]))
    try:
        lease.prepare()
    except Exception as error:
        write_json(directory / "window-ack.json", {"version": 1, "success": False, "error": str(error)})
        write_json(directory / "window-setup.json", lease.audit)
        raise
    write_json(directory / "window-ack.json", {"version": 1, "success": True})
    return lease


def capture_window(request, directory, fixture):
    require(request["version"] == 1 and request["symbol"] == fixture.entry,
            "Capture checkpoint belongs to another function")
    require(request["generation"] == fixture.generation, "Capture checkpoint has another generation")
    require_viewport(request["viewport"])
    window = request["window_id"]
    require(isinstance(window, int) and window > 0, "Native checkpoint has no owned window ID")
    return capture_image(window, directory / "native.png")


def capture_image(window, image):
    result = subprocess.run(["/usr/sbin/screencapture", "-x", "-l", str(window), str(image)],
                            capture_output=True, text=True, timeout=15)
    require(result.returncode == 0 and image.is_file(),
            f"Owned-window capture unavailable: {result.stderr.strip()}")
    raw = image.read_bytes()
    require(raw[:8] == b"\x89PNG\r\n\x1a\n" and len(raw) >= 24, "Capture is not a PNG")
    dimensions = struct.unpack(">II", raw[16:24])
    require(dimensions[0] >= 1050 and dimensions[1] >= 680, "Native capture is smaller than minimum window")
    return {"path": str(image), "dimensions": dimensions,
            "sha256": hashlib.sha256(raw).hexdigest(), "window_id": window}


def require_viewport(actual):
    require(len(actual) == 2 and all(abs(observed - expected) <= 1.0
                                   for observed, expected in zip(actual, VIEWPORT)),
            f"Native content viewport {actual} does not match required minimum {VIEWPORT}")


def require_bundled_runtime(report, app):
    runtime = report["runtime"]
    expected = {
        "engine": app / "Contents/MacOS/codecallnavigator-engine",
        "resources": app / "Contents/Resources",
        "dot": app / "Contents/Resources/analyzers/graphviz/bin/dot",
    }
    for name, path in expected.items():
        require(Path(runtime[name]).resolve() == path.resolve(),
                f"Native {name} escaped relocated bundle: {runtime[name]}")


def stop_owned_process(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


def cleanup_native(process, lease, directory):
    try:
        if lease is not None:
            lease.restore(process)
    finally:
        stop_owned_process(process)
        if lease is not None:
            write_json(directory / "window-setup.json", lease.audit)


def run_native(run):
    app, fixture, directory = run.app, run.fixture, run.directory
    directory.mkdir(parents=True)
    write_json(directory / "config.json", {
        "version": 1, "project": str(fixture.project), "root_symbol": fixture.entry,
        "target_symbol": fixture.target, "source": fixture.source, "timeout_seconds": run.timeout,
        "expected_viewport": VIEWPORT,
    })
    environment = clean_environment()
    environment.update(CODECALLNAVIGATOR_DATA_DIR=str(run.store),
                       CODECALLNAVIGATOR_FLOW_ACCEPTANCE=str(directory), PATH="/usr/bin:/bin")
    command = [str(app / "Contents/MacOS/CodeCallNavigator"), "--native", "--trace",
               str(directory / "native.jsonl"), "--size", f"{VIEWPORT[0]}x{VIEWPORT[1]}", "--theme", run.theme]
    with (directory / "app.log").open("wb") as log:
        process = subprocess.Popen(command, env=environment, stdout=log, stderr=subprocess.STDOUT)
        lease = None
        try:
            lease = setup_window(process, directory, run.timeout)
            request = wait_request(process, directory, "capture-request.json", time.monotonic() + run.timeout)
            try:
                capture = capture_window(request, directory, fixture)
            except Exception as error:
                write_json(directory / "capture-ack.json", {"version": 1, "success": False, "error": str(error)})
                raise
            write_json(directory / "capture-ack.json", {"version": 1, "success": True})
            require(process.wait(timeout=30) == 0, "Native app returned failure")
            report = json.loads((directory / "report.json").read_text())
            require(report["version"] == 1 and report["status"] == "passed", f"Native acceptance failed: {report}")
            require_viewport(report["viewport"])
            require(tuple(report["expected_viewport"]) == VIEWPORT, "Probe used another viewport expectation")
            require_bundled_runtime(report, app)
            return {"theme": run.theme, "command": command, "capture": capture,
                    "observations": report, "window_setup": lease.audit}
        finally:
            cleanup_native(process, lease, directory)


def acceptance(app, output, timeout):
    output.mkdir(parents=True, exist_ok=True)
    require(not any((output / theme).exists() for theme in ("dark", "light")),
            "Use a new output directory; existing native evidence is preserved")
    with tempfile.TemporaryDirectory(prefix="csharp-desktop-", dir="/private/tmp") as temporary:
        root = Path(temporary)
        relocated = root / "Relocated CodeCallNavigator.app"
        shutil.copytree(app, relocated, symlinks=True)
        fixture = prepare_fixture(relocated / "Contents/MacOS/codecallnavigator-engine", root / "project")
        results = []
        for theme in ("dark", "light"):
            store = root / f"store-{theme}"
            seed_session(store, fixture)
            results.append(run_native(NativeRun(relocated, fixture, store, output / theme, theme, timeout)))
        return {"version": 1, "status": "passed", "metrics": fixture.metrics, "native": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
    require(30 <= args.timeout <= 300, "Timeout must be between30 and300seconds")
    output = args.output.resolve()
    try:
        report = acceptance(args.app.resolve(), output, args.timeout)
    except Exception as error:
        output.mkdir(parents=True, exist_ok=True)
        write_json(output / "result.json", {"version": 1, "status": "failed", "error": str(error)})
        raise
    write_json(output / "result.json", report)
    print(f"Native C# desktop acceptance passed: {output}")


if __name__ == "__main__":
    main()
