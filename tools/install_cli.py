#!/usr/bin/env python3
"""Install this complete CodeCallNavigatorEngine distribution for terminal use."""
import argparse
from contextlib import contextmanager
import fcntl
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile


PACKAGE = 'CodeCallNavigatorEngine'
MARKER = '.ccnavigator-install'
OWNER = 'ccnavigator user installer v1\n'


def exists(path):
    return path.exists() or path.is_symlink()


def regular_file(path):
    return not path.is_symlink() and path.is_file()


def check_owned_directory(destination):
    if not exists(destination):
        return
    if destination.is_symlink() or not destination.is_dir():
        raise RuntimeError(f'Refusing unrelated installation directory: {destination}')
    marker = destination / MARKER
    if not regular_file(marker) or marker.read_text() != OWNER:
        raise RuntimeError(f'Refusing unrelated installation directory: {destination}')


def check_destination(destination, command):
    check_owned_directory(destination)
    if not exists(command):
        return
    target = destination / 'bin/ccnavigator'
    if not command.is_symlink() or command.readlink() != target:
        raise RuntimeError(f'Refusing unrelated terminal command: {command}')


def check_executable(binary):
    if not regular_file(binary) or not os.access(binary, os.X_OK):
        raise RuntimeError(f'Missing executable in distribution: {binary}')


def validate(package):
    for name in ['ccnavigator', 'codecallnavigator-engine']:
        check_executable(package / 'bin' / name)
    if not (package / 'Resources/analyzers').is_dir():
        raise RuntimeError(f'Missing analyzer resources: {package / "Resources/analyzers"}')
    result = subprocess.run([str(package / 'bin/ccnavigator'), 'capabilities'],
                            capture_output=True, text=True, check=True, timeout=30)
    capabilities = json.loads(result.stdout)
    if capabilities.get('query_version') != 1 or not capabilities.get('generation_binding'):
        raise RuntimeError('Distribution has incompatible CLI capabilities')


@contextmanager
def installation_lock(share):
    path = share / '.ccnavigator-install.lock'
    descriptor = os.open(path, os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        yield


def publish(staged, destination, command, temporary):
    previous = temporary / 'previous'
    link = temporary / 'command'
    link.symlink_to(destination / 'bin/ccnavigator')
    if destination.exists():
        os.replace(destination, previous)
    try:
        os.replace(staged, destination)
        os.replace(link, command)
    except BaseException:
        # The command link is replaced last. Until then it still points to the
        # original location, which is restored before propagating any failure.
        if destination.exists():
            os.replace(destination, temporary / 'failed')
        if previous.exists():
            os.replace(previous, destination)
        raise


@contextmanager
def staging_directory(share):
    temporary = Path(tempfile.mkdtemp(prefix='.ccnavigator-stage-', dir=share))
    try:
        yield temporary
    except BaseException as error:
        previous = temporary / 'previous'
        if previous.exists():
            raise RuntimeError(f'Rollback failed; previous installation preserved at {previous}') from error
        shutil.rmtree(temporary, ignore_errors=True)
        raise
    else:
        shutil.rmtree(temporary, ignore_errors=True)


def prepare_layout(source, prefix):
    share, bins = prefix / 'share', prefix / 'bin'
    destination = share / PACKAGE
    if share.is_relative_to(source):
        raise RuntimeError('Installation prefix and source distribution must not overlap')
    if source != destination and source.is_relative_to(destination):
        raise RuntimeError('Installation prefix and source distribution must not overlap')
    for directory in [share, bins]:
        prepare_directory(directory)
    return share, destination, bins / 'ccnavigator'


def prepare_directory(directory):
    if directory.is_symlink():
        raise RuntimeError(f'Installation directory must not be a symlink: {directory}')
    directory.mkdir(parents=True, exist_ok=True)


def check_source(source):
    for path in source.rglob('*'):
        if path.is_symlink():
            raise RuntimeError(f'Distribution must not contain symlinks: {path}')


def install(source, prefix):
    source, prefix = Path(source).resolve(), Path(prefix).expanduser().resolve()
    share, destination, command = prepare_layout(source, prefix)
    with installation_lock(share):
        check_destination(destination, command)
        check_source(source)
        with staging_directory(share) as temporary:
            staged = temporary / PACKAGE
            shutil.copytree(source, staged, symlinks=True)
            validate(staged)
            (staged / MARKER).write_text(OWNER)
            publish(staged, destination, command, temporary)
    return command


def report_installation(command):
    print(f'Installed terminal command: {command}')
    paths = [Path(value).expanduser().resolve() for value in os.get_exec_path()]
    if command.parent not in paths:
        print('Add this directory to your shell PATH, then open a new terminal:')
        print(f'export PATH={shlex.quote(str(command.parent))}:"$PATH"')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, default=Path.home() / '.local',
                        help='Install under PREFIX/bin and PREFIX/share (default: ~/.local)')
    args = parser.parse_args()
    command = install(Path(__file__).resolve().parent, args.prefix)
    report_installation(command)


if __name__ == '__main__':
    try:
        main()
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit(f'CLI installation failed: {error}') from error
