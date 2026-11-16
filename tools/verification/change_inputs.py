"""Read Git change boundaries without modifying the checkout or fetching refs."""
from dataclasses import dataclass
import os
from pathlib import PurePosixPath
import subprocess


@dataclass(frozen=True, order=True)
class Change:
    path: str
    status: str
    origin: str


def git(root, *arguments):
    return subprocess.check_output(['git', *arguments], cwd=root,
                                   env=os.environ | {'GIT_OPTIONAL_LOCKS': '0'})


def checked_path(path):
    value = PurePosixPath(path)
    if any((not path, value.is_absolute(), '..' in value.parts, '\0' in path)):
        raise ValueError(f'Invalid repository-relative path: {path!r}')
    return path


def parse_changes(data, origin):
    fields = os.fsdecode(data).split('\0')
    rows = []
    index = 0
    while index < len(fields) and fields[index]:
        status = fields[index]
        count = 2 if status.startswith(('R', 'C')) else 1
        paths = fields[index + 1:index + 1 + count]
        if len(paths) != count or not all(paths):
            raise ValueError('Incomplete Git name-status response')
        rows.extend(Change(checked_path(path), status, origin) for path in paths)
        index += count + 1
    return rows


def discover_changes(root, base):
    revision = git(root, 'rev-parse', '--verify', '--end-of-options', base + '^{commit}').decode().strip()
    options = ('--no-ext-diff', '--no-textconv', '--ignore-submodules=none',
               '--name-status', '-z', '--find-renames')
    rows = parse_changes(git(root, 'diff', '--cached', *options, revision, '--'), 'index')
    rows += parse_changes(git(root, 'diff', *options, '--'), 'worktree')
    rows += [Change(checked_path(path), '?', 'untracked') for path in
             os.fsdecode(git(root, 'ls-files', '--others', '--exclude-standard', '-z')).split('\0') if path]
    return revision, sorted(set(rows))


def inventory_at(root, revision, path):
    """Distinguish absent paths from failing object reads, including the index."""
    if revision == ':':
        entries = git(root, 'ls-files', '--stage', '-z', '--', path)
        specification = ':' + path
    else:
        entries = git(root, 'ls-tree', '-z', '--full-tree', revision, '--', path)
        specification = f'{revision}:{path}'
    if not entries:
        return None
    return git(root, 'show', specification)
