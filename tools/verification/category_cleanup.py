#!/usr/bin/env python3
"""Clean explicitly owned category artifacts while retaining verification evidence."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verification.cleanup import Action, Settings, backup_results, check_usage, perform, require, safe_path


CATEGORIES = ('unit', 'integration', 'acceptance', 'performance')
MARKER = 'category-workspace.json'
ARTIFACTS = ('build', 'profile', 'test-store', 'scale-cache', 'tmp')


def temporary_path(root, output):
    """Short socket-safe path outside the repository and its Cargo workspace."""
    identity = f'{Path(root).resolve()}\0{Path(output).resolve()}'.encode()
    token = hashlib.sha256(identity).hexdigest()[:16]
    parent = Path('/private/tmp') if sys.platform == 'darwin' else Path('/tmp')
    return parent / ('ccn-verify-' + token)


def temporary_owner(root, output, category):
    return dict(version=1, root=str(root), output=str(output), category=category,
                temporary_directory=str(temporary_path(root, output)))


def category_location(root, output):
    root, output = Path(root), Path(output)
    require(not root.is_symlink(), 'Repository root cannot be a symlink')
    root = root.resolve(strict=True)
    require(root.is_dir() and root != Path(root.anchor), 'Select a repository directory')
    output = safe_path(root, output)
    parent = safe_path(root, root / 'target/verification')
    require(output.is_relative_to(parent) and output != parent,
            'Category output must be beneath target/verification')
    return root, output


def category_paths(root, output):
    root, output = category_location(root, output)
    require(output.is_dir(), 'Category output must be an existing evidence directory')
    results = safe_path(output, output / 'results.json')
    require(results.is_file() and isinstance(json.loads(results.read_text()), list),
            'Category output requires results.json ownership evidence')
    return root, output


def prepare_category(root, output, category):
    """Called under the checkout verification lock, before any category build."""
    require(category in CATEGORIES, 'Unknown test category')
    root, output = category_paths(root, output)
    marker = safe_path(output, output / MARKER)
    require(not marker.exists(), 'Category workspace is already owned')
    data = temporary_owner(root, output, category)
    temporary = temporary_path(root, output)
    # Never adopt a collision, even if an existing directory claims this owner.
    temporary.mkdir(mode=0o700)
    with (temporary / MARKER).open('x') as stream:
        stream.write(json.dumps(data, indent=2) + '\n')
    # Exclusive creation cannot replace a marker raced into the evidence directory.
    with marker.open('x') as stream:
        stream.write(json.dumps(data, indent=2) + '\n')


def initialize_category(root, output, category):
    """Create fresh evidence and a short external temporary workspace."""
    root, output = category_location(root, output)
    output.mkdir(parents=True, exist_ok=False)
    (output / 'results.json').write_text('[]\n')
    prepare_category(root, output, category)
    # Older already-planned composites still point TMPDIR here. Keep the empty
    # owned directory until those runs finish; new plans use temporary_path().
    (output / 'tmp').mkdir()


def owned_category(root, output):
    root, output = category_paths(root, output)
    marker = safe_path(output, output / MARKER)
    require(marker.is_file(), 'Category workspace ownership marker is missing')
    data = json.loads(marker.read_text())
    require(isinstance(data, dict) and data.get('version') == 1
            and data.get('root') == str(root) and data.get('output') == str(output)
            and data.get('category') in CATEGORIES,
            'Category workspace ownership marker does not match this run')
    return root, output, data['category']


def reclaimed_bytes(actions):
    return sum(max(0, row['estimated_bytes_before'] - row['estimated_bytes_after'])
               for row in actions if row.get('estimated_bytes_before') is not None
               and row.get('estimated_bytes_after') is not None)


def cleanup_if_prepared(root, output, keep_artifacts=False, checker=check_usage):
    root, output = category_location(root, output)
    if not output.exists():
        return dict(status='passed', output=str(output), reason='Category was not prepared',
                    actions=[], errors=[], bytes_reclaimed=0, bytes_retained=0)
    return cleanup_category(root, output, keep_artifacts, checker)


def action_summary(actions):
    retained = sum(row['estimated_bytes_after'] or 0 for row in actions)
    measured = {row['path'] for row in actions if row['estimated_bytes_after'] is not None}
    unmeasured = sorted({row['path'] for row in actions} - measured)
    good = {'removed', 'absent', 'retained'}
    return dict(bytes_reclaimed=reclaimed_bytes(actions), bytes_retained=retained,
                unmeasured_paths=unmeasured,
                status='passed' if all(row['status'] in good for row in actions) else 'failed')


def external_action(root, output, category):
    data = json.loads(safe_path(output, output / MARKER).read_text())
    if 'temporary_directory' not in data:
        return None  # Legacy runs own only their repository-local artifacts.
    expected = temporary_owner(root, output, category)
    require(data == expected, 'External temporary workspace binding does not match this run')
    temporary = temporary_path(root, output)
    require(not temporary.is_symlink(), 'External temporary workspace cannot be a symlink')
    if temporary.exists():
        safe_path(temporary, temporary)
        marker = safe_path(temporary, temporary / MARKER)
        require(marker.is_file() and json.loads(marker.read_text()) == expected,
                'External temporary workspace ownership marker does not match this run')
    return Action(temporary, owner=temporary)


def category_actions(root, output, category):
    external = external_action(root, output, category)
    actions = [Action(output / name, owner=output) for name in ARTIFACTS]
    if external is not None:
        actions.append(external)
    return actions


def category_results(settings, paths, category, checker):
    root, output = paths
    actions = category_actions(root, output, category)
    native = native_retention_results(settings, category, checker)
    if any(row['status'] == 'failed' for row in native):
        # Preserve owned workspaces if native diagnostics cannot be relocated.
        # Their sizes remain visible even when safety prevents all deletion.
        retained = Settings(root, output, 'light', True)
        return native + [perform(retained, paths, action, checker) for action in actions]
    rows = [perform(settings, paths, action, checker) for action in actions]
    if category == 'acceptance':
        # Package retention is a category-return responsibility too, but it must
        # not make an already-cleaned category appear to have build artifacts.
        rows.extend(backup_results(settings, paths, checker))
    return native + rows


def native_retention_results(settings, category, checker):
    if category != 'acceptance' or settings.keep_artifacts:
        return []
    from native_fixture import retain_native_fixtures
    return retain_native_fixtures(settings.root, settings.output, checker)


def cleanup_category(root, output, keep_artifacts=False, checker=check_usage):
    report = dict(status='failed', actions=[], errors=[], bytes_reclaimed=0, bytes_retained=0,
                  keep_artifacts=keep_artifacts)
    destination = None
    try:
        root, output, category = owned_category(root, output)
        destination = safe_path(output, output / 'category-cleanup.json')
        report.update(category=category, output=str(output))
        settings = Settings(root, output, 'light', keep_artifacts)
        report['actions'] = category_results(settings, (root, output), category, checker)
        report.update(action_summary(report['actions']))
    except (OSError, RuntimeError, ValueError) as error:
        report['errors'].append(str(error))
    if destination is not None:
        try:
            destination.write_text(json.dumps(report, indent=2) + '\n')
        except OSError as error:
            report['status'] = 'failed'
            report['errors'].append(str(error))
    return report


def reclaim_abandoned(root, current_output, keep_artifacts=False, checker=check_usage):
    """Under the checkout lock, reclaim only marked, proven-idle old workspaces.

    Evidence directories themselves are retained. Ambiguous inspection fails closed;
    it never authorizes deleting an older build just because this run needs space.
    """
    root, current_output = category_paths(root, current_output)
    reports = []
    parent = safe_path(root, root / 'target/verification')
    outputs = marked_workspaces(parent)
    reports = [reclaim_one(root, output, keep_artifacts, checker)
               for output in outputs if output != current_output]
    return dict(status='passed' if all(row['status'] == 'passed' for row in reports)
                else 'failed', workspaces=reports,
                bytes_reclaimed=sum(row.get('bytes_reclaimed', 0) for row in reports),
                bytes_retained=sum(row.get('bytes_retained', 0) for row in reports))


def marked_workspaces(parent):
    """Never interpret miniature verification trees inside test/build artifacts."""
    for directory, names, files in os.walk(parent, followlinks=False):
        path = Path(directory)
        names[:] = sorted(name for name in names
                          if name not in ARTIFACTS and not (path / name).is_symlink())
        if MARKER in files:
            yield path


def reclaim_one(root, output, keep_artifacts, checker):
    try:
        _, output, category = owned_category(root, output)
        if artifacts_absent(root, output, category):
            # Keep the first cleanup's historical byte accounting intact. A later
            # category discovering no artifacts reclaimed nothing itself.
            return dict(status='passed', output=str(output), reason='Already cleaned',
                        bytes_reclaimed=0, bytes_retained=0)
        keep_artifacts = keep_artifacts or previously_retained(output)
        if not keep_artifacts:
            usage = checker(output)
            require(usage.status == 'idle', usage.reason or 'Old category is not proven idle')
        return cleanup_category(root, output, keep_artifacts, checker)
    except (OSError, RuntimeError, ValueError) as error:
        return dict(status='failed', output=str(output), errors=[str(error)])


def artifacts_absent(root, output, category):
    actions = category_actions(root, output, category)
    # Validate every owned path before declaring the workspace empty; dangling or
    # internal symlinks must never be mistaken for absent artifacts.
    paths = [safe_path(action.owner or root, action.path) for action in actions]
    return all(not path.exists() for path in paths)


def previously_retained(output):
    path = safe_path(output, output / 'category-cleanup.json')
    if not path.exists():
        return False
    report = json.loads(path.read_text())
    require(isinstance(report, dict), 'Previous category cleanup report is invalid')
    return report.get('keep_artifacts') is True


def prepare_requested(args):
    operation = initialize_category if args.initialize else prepare_category
    operation(args.root, args.output, args.prepare)


def requested_report(args):
    if args.reclaim_abandoned:
        return reclaim_abandoned(args.root, args.output, args.keep_artifacts)
    operation = cleanup_if_prepared if args.if_unprepared else cleanup_category
    return operation(args.root, args.output, args.keep_artifacts)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--keep-artifacts', action='store_true')
    parser.add_argument('--prepare', choices=CATEGORIES)
    parser.add_argument('--initialize', action='store_true')
    parser.add_argument('--reclaim-abandoned', action='store_true')
    parser.add_argument('--if-unprepared', action='store_true',
                        help='Allow an absent category after an earlier stage failed')
    args = parser.parse_args()
    if args.prepare:
        prepare_requested(args)
        return 0
    report = requested_report(args)
    print(json.dumps(report, indent=2))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
