#!/usr/bin/env python3
"""Remove only idle verification artifacts, preserving evidence and shared caches."""
from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import argparse
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys

if __package__:
    from .runner import Step, execute
else:
    from runner import Step, execute


@dataclass(frozen=True)
class Settings:
    root: Path
    output: Path
    mode: str
    keep_artifacts: bool = False
    cargo_command: str = 'cargo'


@dataclass(frozen=True)
class Usage:
    status: str
    reason: str = ''


class Operation(Enum):
    REMOVE = 'remove'
    DEV = 'cargo-dev'
    RELEASE = 'cargo-release'
    RETAIN = 'retain'


@dataclass(frozen=True)
class Action:
    path: Path
    operation: Operation = Operation.REMOVE
    reason: str = ''
    owner: Path | None = None


class CleanupError(RuntimeError):
    pass


def require(condition, message):
    if not condition:
        raise CleanupError(message)


def safe_path(root, candidate):
    """Reject symlinks in every component below the explicitly selected repository."""
    candidate = Path(os.path.abspath(candidate))
    require(candidate.resolve().is_relative_to(root), f'Cleanup path escapes repository: {candidate}')
    current = candidate
    while current.resolve() != root:
        require(not current.is_symlink(), f'Cleanup path contains a symlink: {current}')
        current = current.parent
    require(not current.is_symlink(), f'Repository path is a symlink: {current}')
    return candidate.resolve()


def context(settings):
    require(settings.mode in ('full', 'light'), 'Unknown cleanup mode')
    require(not settings.root.is_symlink(), 'Repository root cannot be a symlink')
    root = settings.root.resolve(strict=True)
    require(root.is_dir() and root != Path(root.anchor), 'Select a repository directory')
    require(not settings.output.is_symlink(), 'Run output cannot be a symlink')
    output = settings.output.resolve(strict=True)
    require(output != root and output.is_dir(), 'Cleanup output must be an owned run directory')
    marker = safe_path(output, output / 'results.json')
    require(marker.is_file(), 'Cleanup requires this run results.json ownership marker')
    results = json.loads(marker.read_text())
    require(isinstance(results, list), 'Run results.json must contain verification stages')
    safe_path(output, output / 'cleanup.json')
    return root, output, results


def check_usage(path):
    """Only a native successful lsof query can establish that a directory is idle."""
    if sys.platform != 'darwin':
        return Usage('unavailable', 'Native macOS lsof process inspection is unavailable')
    try:
        result = subprocess.run(
            ['/usr/sbin/lsof', '-nP', '-Fp', '+D', str(path)],
            capture_output=True, text=True, timeout=30, check=False,
        )
        return lsof_usage(result)
    except (OSError, subprocess.TimeoutExpired):
        return Usage('unavailable', 'Native lsof failed or timed out')


def lsof_usage(result):
    if result.stderr.strip():
        return Usage('unavailable', 'Native lsof reported an inspection error')
    if result.returncode == 1 and not result.stdout.strip():
        return Usage('idle')
    if result.returncode == 0 and result.stdout.strip():
        return Usage('active', 'An active process has files open in this directory')
    return Usage('unavailable', 'Native lsof did not return a conclusive process result')


def tree_bytes(path):
    """Estimate logical bytes by lstat; nested symbolic links are never traversed."""
    pending, total = [path], 0
    while pending:
        current = pending.pop()
        metadata = current.lstat()
        total += metadata.st_size
        if stat.S_ISDIR(metadata.st_mode):
            with os.scandir(current) as entries:
                pending.extend(Path(entry.path) for entry in entries)
    return total


def directory_bytes(path):
    return tree_bytes(path) if path.exists() else 0


def build_was_reached(results):
    stages = {'bootstrap', 'cargo-check', 'clippy', 'rust-tests',
              'release-acceptance-and-benchmarks', 'storage-benchmark', 'package'}
    return any(stage_reached(stage, stages) for stage in results)


def stage_reached(stage, names):
    require(isinstance(stage, dict), 'Run results contain an invalid stage')
    name = stage.get('name', '')
    is_build = name in names or name.startswith('build-')
    return is_build and stage.get('status') not in (None, 'not-run', 'skipped')


def build_actions(root, mode, reached):
    if mode == 'light':
        return [Action(root / f'target/{profile}/incremental') for profile in ('debug', 'release')]
    if not reached:
        return [Action(root / f'target/{profile}', Operation.RETAIN,
                       'Build stages were not reached; preserve existing compiled artifacts')
                for profile in ('debug', 'release')]
    return [Action(root / 'target/debug', Operation.DEV),
            Action(root / 'target/release', Operation.RELEASE)]


def backup_actions(root):
    dist = safe_path(root, root / 'dist')
    if not dist.exists():
        return []
    require(dist.is_dir(), 'Package directory must be a directory')
    return [action for prefix in ('CodeCallNavigator', 'CodeGraph')
            for action in previous_packages(dist, prefix)]


def previous_packages(dist, prefix):
    candidates = sorted(dist.glob(f'{prefix}.previous-*.app'),
                        key=lambda path: (-path.lstat().st_mtime_ns, path.name))
    valid = [item for item in candidates if stat.S_ISDIR(item.lstat().st_mode)]
    newest = valid[0] if valid else None
    return [Action(path, Operation.RETAIN, 'Keep newest previous package')
            if path == newest else Action(path) for path in candidates]


def initial_action(action):
    return dict(path=str(action.path), operation=action.operation.value,
                status='not-run', estimated_bytes_before=None, estimated_bytes_after=None)


def validate_directory(root, path):
    safe_path(root, path)
    require(path.is_dir(), f'Cleanup candidate is not a directory: {path}')


def inspect_action(root, action, result):
    safe_path(root, action.path)
    if not action.path.exists():
        result.update(status='absent', estimated_bytes_before=0, estimated_bytes_after=0)
        return False
    validate_directory(root, action.path)
    size = tree_bytes(action.path)
    result.update(estimated_bytes_before=size, estimated_bytes_after=size)
    return True


def available(checker, action, result):
    usage = checker(action.path)
    if usage.status == 'idle':
        return True
    result.update(status='blocked' if usage.status == 'active' else 'unavailable', reason=usage.reason)
    return False


def cargo_clean(settings, root, output, action):
    command = [settings.cargo_command, 'clean', '--locked', '--target-dir', str(root / 'target')]
    if action.operation == Operation.DEV:
        command.extend(['--profile', 'dev'])
    else:
        command.append('--release')
    log_path = safe_path(output, output / f'cleanup-{action.operation.value}.log')
    with log_path.open('a') as log:
        environment = {'CARGO_TARGET_DIR': str(root / 'target'),
                       'CARGO_BUILD_BUILD_DIR': str(root / 'target')}
        status, code = execute(Step(action.operation.value, command, 120, environment), root, log)
    require(status == 'passed', f'{action.operation.value} {status}; inspect {log_path.name} (exit {code})')


def mutate(settings, paths, action):
    root, output = paths
    validate_directory(action.owner or root, action.path)
    require(not output.is_relative_to(action.path), 'Cleanup cannot remove its own run evidence')
    if action.operation == Operation.REMOVE:
        shutil.rmtree(action.path)
        return 'removed'
    cargo_clean(settings, root, output, action)
    return 'cleaned'


def perform(settings, paths, action, checker):
    root, _ = paths
    result = initial_action(action)
    try:
        if not inspect_action(action.owner or root, action, result):
            return result
        if retain_action(settings, action, result):
            return result
        if available(checker, action, result):
            result['status'] = mutate(settings, paths, action)
            result['estimated_bytes_after'] = directory_bytes(action.path)
    except (CleanupError, OSError, subprocess.SubprocessError) as error:
        result.update(status='failed', reason=str(error))
    finally:
        update_remaining(action.owner or root, action, result)
    return result


def retain_action(settings, action, result):
    if not (settings.keep_artifacts or action.operation == Operation.RETAIN):
        return False
    result.update(status='retained', reason=action.reason or 'Explicit --keep-artifacts')
    return True


def update_remaining(owner, action, result):
    if result['estimated_bytes_before'] is None:
        return
    try:
        result['estimated_bytes_after'] = directory_bytes(safe_path(owner, action.path))
    except (CleanupError, OSError) as error:
        result.update(status='failed', estimated_bytes_after=None, reason=str(error))


def write_report(output, report):
    destination = safe_path(output, output / 'cleanup.json')
    destination.write_text(json.dumps(report, indent=2) + '\n')


def execute_actions(settings, paths, results, checker):
    root, output = paths
    actions = build_actions(root, settings.mode, build_was_reached(results))
    actions += [Action(output / name, owner=output) for name in ('profile', 'scale-cache')]
    rows = [perform(settings, paths, action, checker) for action in actions]
    return rows + backup_results(settings, paths, checker)


def backup_results(settings, paths, checker):
    root, _ = paths
    try:
        return [perform(settings, paths, action, checker) for action in backup_actions(root)]
    except (CleanupError, OSError) as error:
        return [dict(path=str(root / 'dist'), operation='prune-previous-packages',
                     status='failed', reason=str(error),
                     estimated_bytes_before=None, estimated_bytes_after=None)]


def save_report(output, report):
    if output is None:
        return
    try:
        write_report(output, report)
    except (CleanupError, OSError) as error:
        report['errors'].append(str(error))
        report['status'] = 'failed'


def cleanup_report(settings, checker=check_usage):
    report = dict(status='failed', mode=settings.mode, keep_artifacts=settings.keep_artifacts,
                  actions=[], errors=[])
    output = None
    try:
        root, output, results = context(settings)
        report['actions'] = execute_actions(settings, (root, output), results, checker)
        good = {'removed', 'cleaned', 'absent', 'retained'}
        report['status'] = 'passed' if all(row['status'] in good for row in report['actions']) else 'failed'
    except (CleanupError, OSError, ValueError) as error:
        report['errors'].append(str(error))
    save_report(output, report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--mode', choices=('full', 'light'), required=True)
    parser.add_argument('--keep-artifacts', action='store_true')
    args = parser.parse_args()
    settings = Settings(args.root, args.output, args.mode, args.keep_artifacts)
    report = cleanup_report(settings)
    print(json.dumps(report, indent=2))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
