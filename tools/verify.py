#!/usr/bin/env python3
"""Run local full/light verification with bounded stages and durable evidence."""
import argparse
import json
import os
from pathlib import Path
import sys
import signal
import subprocess
import tempfile

from verification.checks import changed_files, docs_only, static_steps, runtime_steps, full_steps
from verification.runner import Runner, Step, exclusive

ROOT = Path(__file__).resolve().parents[1]


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["full", "light"])
    parser.add_argument("--base", default="HEAD", help="Diff base for formatting and docs-only classification; include staged, unstaged and untracked files")
    parser.add_argument("--profile-source", type=Path, default=os.environ.get("CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE"), help="Saved application data directory to snapshot for full benchmarks")
    parser.add_argument("--output", type=Path, help="New evidence directory (default: unique directory under target/verification)")
    parser.add_argument("--timeout", type=int, default=1800, help="Maximum seconds per build/test/package stage (default: 1800)")
    parser.add_argument("--codescene-command", default="cs-mcp", help="CodeScene MCP executable")
    parser.add_argument("--keep-artifacts", action="store_true", help="Retain build/profiling artifacts and rotated packages for investigation")
    parser.add_argument("--dry-run", action="store_true", help="Print exact stages without executing or creating evidence")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    if args.profile_source is not None:
        args.profile_source = args.profile_source.resolve()
    if os.path.sep in args.codescene_command:
        args.codescene_command = str(Path(args.codescene_command).resolve())
    return args


def environment(root, output):
    values = {"CARGO_TARGET_DIR": str(root / "target"),
              "CODECALLNAVIGATOR_ENGINE": str(root / "target/debug/codecallnavigator-engine"),
              "CODECALLNAVIGATOR_TEST_ENGINE": str(root / "target/debug/codecallnavigator-engine")}
    values["CODECALLNAVIGATOR_DATA_DIR"] = str(output / "test-store")
    values["CARGO_BUILD_BUILD_DIR"] = str(root / "target")
    values["CARGO_INCREMENTAL"] = "0"
    values["CARGO_CACHE_AUTO_CLEAN_FREQUENCY"] = "always"
    for key in ["CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT", "CODECALLNAVIGATOR_RECURRING_BILLING", "CODECALLNAVIGATOR_TEST_MAVEN"]:
        if os.environ.get(key):
            values[key] = str(Path(os.environ[key]).resolve())
    modules = os.environ.get("CODECALLNAVIGATOR_TS_NODE_MODULES")
    if modules:
        values["CODECALLNAVIGATOR_TS_NODE_MODULES"] = str(Path(modules).resolve())
        values.setdefault("CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT", str(Path(modules).resolve() / "typescript"))
    return values


def quality_step(args, paths):
    files = [p for p in paths if (ROOT / p).is_file()]
    command = [sys.executable, "tools/verification/quality.py", "--root", str(ROOT),
               "--command", args.codescene_command, "--timeout", "120", "--files", *files]
    return Step("codescene", command, 180)


def prerequisite_step(args):
    command = [sys.executable, "tools/verification/prerequisites.py", "--root", str(ROOT),
               "--codescene-command", args.codescene_command]
    if args.mode == "light":
        command.append("--light")
    elif args.profile_source:
        command.extend(["--profile-source", str(args.profile_source.resolve())])
    return Step("prerequisites", command, 180)


def cleanup_step(args, output):
    command = [sys.executable, "tools/verification/cleanup.py", "--root", str(ROOT),
               "--output", str(output), "--mode", args.mode]
    if getattr(args, "keep_artifacts", False):
        command.append("--keep-artifacts")
    return Step("cleanup", command, 600, always=True)


def plan(args, paths, output):
    python = sys.executable
    runtime = args.mode == "full" or not docs_only(paths)
    steps = static_steps(ROOT, paths, args.base, python)
    if not runtime:
        return steps + [cleanup_step(args, output)]
    quality = quality_step(args, paths)
    steps.insert(0, prerequisite_step(args))
    if args.mode == "full":
        steps.append(Step("bootstrap", [python, "tools/bootstrap.py"], args.timeout))
    steps.extend(runtime_steps(ROOT, python, args.timeout))
    if args.mode == "full":
        steps.extend(full_steps(ROOT, args, output))
    steps.append(quality)
    steps.append(cleanup_step(args, output))
    for step in steps:
        step.environment = environment(ROOT, output) | step.environment
    return steps


def main():
    args = arguments()
    paths = changed_files(ROOT, args.base)
    if args.dry_run:
        output = (args.output or ROOT / "target/verification/DRY-RUN").resolve()
        print(json.dumps([dict(name=s.name, command=s.command, environment=s.environment,
                               timeout=s.timeout, always=s.always) for s in plan(args, paths, output)], indent=2))
        return 0
    if args.output:
        output = args.output.resolve()
    else:
        parent = ROOT / "target/verification"
        parent.mkdir(parents=True, exist_ok=True)
        output = Path(tempfile.mkdtemp(prefix=args.mode + "-", dir=parent))
        output.rmdir()  # Runner creates a fresh evidence directory atomically.
    runner = Runner(ROOT, output, plan(args, paths, output))
    scope = "documentation-only" if args.mode == "light" and docs_only(paths) else "runtime"
    print(f"{args.mode.capitalize()} verification ({scope}); evidence: {output}", flush=True)
    with exclusive(ROOT):
        result = runner.run()
    (output / "changes.json").write_text(json.dumps(dict(mode=args.mode, base=args.base,
                                                       files=paths, scope=scope), indent=2) + "\n")
    print(f"{args.mode.capitalize()} verification {'PASSED' if result == 0 else 'FAILED'}; {output / 'results.json'}")
    return result


def interrupt(_signal, _frame):
    raise KeyboardInterrupt


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, interrupt)
    try:
        status = main()
    except KeyboardInterrupt:
        print("Verification interrupted before a stage completed", file=sys.stderr)
        status = 130
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        print(f"Verification unavailable: {error}", file=sys.stderr)
        status = 2
    sys.exit(status)
