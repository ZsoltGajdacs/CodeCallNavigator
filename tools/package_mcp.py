#!/usr/bin/env python3
"""Package and verify an MCP-only install without desktop or analyzer resources."""
import argparse
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

from mcp_boundary import inspect
from package import copy_crate_notices, notice_file
from mcp_smoke import smoke
from verification.cleanup import check_usage

ROOT = Path(__file__).resolve().parents[1]


def run(*arguments, **kwargs):
    return subprocess.run([str(value) for value in arguments], check=True, **kwargs)


def source_package(package):
    run('cargo', 'package', '--locked', '--allow-dirty', '--no-verify', '-p', package['name'], cwd=ROOT)
    return ROOT / 'target/package' / f"{package['name']}-{package['version']}.crate"


def install(archive, work):
    with tarfile.open(archive) as contents:
        contents.extractall(work, filter='data')
    manifest = next(work.glob('codecallnavigator-mcp-*/Cargo.toml'))
    destination = work / 'installed'
    build = work / 'build'
    environment = os.environ | {'CARGO_TARGET_DIR': str(build), 'CARGO_BUILD_BUILD_DIR': str(build)}
    run('cargo', 'install', '--locked', '--path', manifest.parent, '--root', destination,
        '--target-dir', build, cwd=work, env=environment)
    return destination / 'bin/codecallnavigator-mcp'


def notices(packages, destination):
    inventory = []
    for package in packages:
        if package['name'] == 'codecallnavigator-mcp':
            continue
        source = Path(package['manifest_path']).parent
        candidates = [path for path in source.iterdir() if notice_file(path)]
        if package.get('license_file'):
            candidates.append(source / package['license_file'])
        copy_crate_notices(candidates, destination / f"{package['name']}-{package['version']}")
        inventory.append({key: package.get(key) for key in ('name', 'version', 'license')})
    destination.mkdir(parents=True, exist_ok=True)
    (destination / 'inventory.json').write_text(json.dumps(inventory, indent=2) + '\n')


def assemble(binary, packages, destination):
    (destination / 'bin').mkdir(parents=True)
    shutil.copy2(binary, destination / 'bin/codecallnavigator-mcp')
    for name in ('LICENSE', 'THIRD_PARTY_NOTICES.md'):
        shutil.copy2(ROOT / name, destination)
    shutil.copy2(ROOT / 'docs/mcp.md', destination / 'README.md')
    notices(packages, destination / 'notices')
    binary = destination / 'bin/codecallnavigator-mcp'
    if platform.system() == 'Darwin':
        run('codesign', '--force', '--sign', '-', binary)
        run('codesign', '--verify', '--strict', binary)
    run(binary, '--help')
    return binary


def publish(source, output):
    destination = output / 'CodeCallNavigatorMcp'
    if destination.exists():
        require_idle(destination)
        previous = output / 'CodeCallNavigatorMcp.previous'
        if previous.exists():
            require_idle(previous)
            shutil.rmtree(previous)
        destination.rename(previous)
    shutil.move(str(source), destination)
    archive = output / 'CodeCallNavigatorMcp.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as contents:
        for path in sorted(destination.rglob('*')):
            contents.write(path, path.relative_to(output))
    return destination


def require_idle(path):
    if path.is_symlink() or not path.is_dir():
        raise RuntimeError(f'Package destination must be an ordinary directory: {path}')
    usage = check_usage(path)
    if usage.status != 'idle':
        raise RuntimeError(f'Cannot rotate an active or unverified MCP package: {path}: {usage.reason}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True, help='Installed CLI for the process smoke test')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    _, packages = inspect(ROOT)
    package = next(value for value in packages if value['name'] == 'codecallnavigator-mcp')
    archive = source_package(package)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    # Extract outside the repository so Cargo cannot inherit its workspace or
    # satisfy an accidental path dependency from a sibling checkout directory.
    with tempfile.TemporaryDirectory(prefix='mcp-package-') as temporary:
        work = Path(temporary)
        binary = install(archive, work)
        staged = work / 'CodeCallNavigatorMcp'
        binary = assemble(binary, packages, staged)
        smoke(binary, args.engine.resolve(), work / 'empty-project')
        destination = publish(staged, output)
    shutil.copy2(archive, output / archive.name)
    print(json.dumps({'package': str(destination), 'independent_install': 'passed', 'stdio_cli': 'passed'}))


if __name__ == '__main__':
    main()
