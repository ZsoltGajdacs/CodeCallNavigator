#!/usr/bin/env python3
"""Run/report native full-detail pan measurements; virtual-clock traces never pass."""
from __future__ import annotations

import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile

from pan_profile_report import read_rows, summarize
from profile_cache import validate_presentation_cache


def snapshot_store(source, destination):
    """A transactionally consistent catalog snapshot, followed by its immutable generation."""
    destination.mkdir()
    with closing(sqlite3.connect((source / "catalog.sqlite3").as_uri() + "?mode=ro", uri=True)) as original:
        with closing(sqlite3.connect(destination / "catalog.sqlite3")) as copy:
            original.backup(copy)
            rows = dict(copy.execute("SELECT key, value FROM catalog"))
    validate_presentation_cache(rows)
    generation = rows.get('active')
    if generation:
        if Path(generation).name != generation:
            raise ValueError("Catalog contains an invalid generation filename")
        shutil.copy2(source / generation, destination / generation)


def binary_hash(binary):
    digest = hashlib.sha256()
    with binary.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def run_profile(args):
    binary, output = args.binary.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    metadata = {"binary": str(binary), "sha256": binary_hash(binary), "scene": args.scene,
                "requested_size": args.size, "expected_native_scale": args.expected_scale}
    (output / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
    with tempfile.TemporaryDirectory(prefix="codecallnavigator-native-pan-") as temporary:
        store = Path(temporary) / "store"
        snapshot_store(args.data_dir.resolve(), store)
        command = [str(binary), "--native", "--scene", args.scene, "--trace", str(output / "native.jsonl"), "--size", args.size]
        environment = os.environ | {"CODECALLNAVIGATOR_DATA_DIR": str(store), "CODECALLNAVIGATOR_PAN_PROFILE": str(output)}
        metadata["command"] = command
        with (output / "process.log").open("w") as log:
            try:
                completed = subprocess.run(command, env=environment, stdout=log, stderr=subprocess.STDOUT, timeout=60, check=False)
                metadata["exit_code"] = completed.returncode
            except subprocess.TimeoutExpired:
                metadata["timed_out"] = True
    metadata["binary_unchanged"] = binary_hash(binary) == metadata["sha256"]
    (output / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
    result = summarize(output, args.expected_scale)
    failed = metadata.get("timed_out") or metadata.get("exit_code") != 0
    if failed or not metadata["binary_unchanged"]:
        result.update(status="unavailable", reason="Native process failed, timed out, or its binary changed; inspect process.log.")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("run", help="Run the packaged release app on the current native display")
    run.add_argument("--binary", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True, help="A new result directory")
    run.add_argument("--data-dir", type=Path, default=Path.home() / "Library/Application Support/CodeCallNavigator")
    run.add_argument("--scene", choices=["saved-canvas", "dense-canvas"], default="saved-canvas")
    run.add_argument("--size", default="1900x1100")
    report = commands.add_parser("report", help="Recompute acceptance from existing native measurements")
    report.add_argument("directory", type=Path)
    for command in [run, report]:
        command.add_argument("--expected-scale", type=float, choices=[1.0, 2.0])
    args = parser.parse_args()
    directory = args.output if args.command == "run" else args.directory
    try:
        result = run_profile(args) if args.command == "run" else summarize(directory, args.expected_scale)
    except (OSError, ValueError, sqlite3.Error) as error:
        result = {"status": "unavailable", "reason": str(error)}
    if directory.is_dir():
        (directory / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0 if result["status"] == "passed" else 2


if __name__ == "__main__":
    sys.exit(main())
