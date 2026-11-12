#!/usr/bin/env python3
"""Validate canonical skills and build deterministic, self-contained host archives."""
import argparse
import json
from pathlib import Path
import re
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
HOSTS = ('claude-code', 'codex')
SKILL = 'skills/graph-context/SKILL.md'
DESCRIPTION = ('Explore CodeCallNavigator symbols, callers, callees, function flow and possible '
               'change impact using configured CodeCallNavigator MCP tools. Use for structural graph '
               'questions, not ordinary text search or unrelated tasks.')
METADATA = {
    'claude-code': ('.claude-plugin/plugin.json', '.claude-plugin/marketplace.json'),
    'codex': ('plugin.json', '.agents/plugins/marketplace.json'),
}


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def canonical(root):
    path = root / 'integrations/shared/graph-context.md'
    require(not any(part.is_symlink() for part in [path, path.parent, path.parent.parent]),
            'Canonical source cannot be a symlink')
    body = path.read_bytes()
    require(body.startswith(b'# Graph context\n') and body.endswith(b'\n'),
            'Canonical skill body must have a title and final newline')
    return body


def wrapper(root):
    header = f'---\nname: graph-context\ndescription: {DESCRIPTION}\n---\n\n'
    return header.encode() + canonical(root)


def allowed_files(host):
    return {SKILL, 'README.md', *METADATA[host]}


def source_files(root, host, missing_skill=False):
    source = root / 'integrations' / host
    require(source.is_dir(), f'Missing host source: {host}')
    require(not source.is_symlink() and not source.parent.is_symlink(),
            f'Host source cannot be a symlink: {host}')
    allowed = allowed_files(host)
    directories = {str(parent) for name in allowed for parent in Path(name).parents}
    files = {}
    for path in sorted(source.rglob('*')):
        require(not path.is_symlink(), f'Source symlink is forbidden: {path}')
        relative = path.relative_to(source).as_posix()
        if path.is_dir():
            require(relative in directories, f'Unexpected source directory: {relative}')
        else:
            require(relative in allowed and path.is_file(), f'Unexpected source file: {relative}')
            files[relative] = path.read_bytes()
    expected = allowed - ({SKILL} if missing_skill else set())
    require(expected <= files.keys(), f'Missing reviewed files: {sorted(expected - files.keys())}')
    return files


def validate_manifest(host, value):
    require(isinstance(value, dict), 'Plugin manifest must be a JSON object')
    fields = {'name', 'version', 'description'}
    if host == 'codex':
        fields.update({'$schema', 'extensions'})
        require(value.get('$schema') == 'https://agent-plugins.org/schemas/1.0.0/plugin.schema.json',
                'Unsupported Codex manifest schema')
        require(value.get('extensions') == {'com.openai': {'requires_local_executor': True}},
                'Codex manifest must require only the local executor extension')
        require(value['extensions']['com.openai']['requires_local_executor'] is True,
                'Codex local executor setting must be boolean true')
    require(set(value) == fields, 'Unexpected manifest fields; hooks and bundled MCP wiring are not packaged')
    require(value.get('name') == 'codecallnavigator', 'Unexpected plugin identity')
    version = value.get('version')
    require(isinstance(version, str) and re.fullmatch(r'\d+\.\d+\.\d+', version),
            'Invalid plugin version')
    require(isinstance(value.get('description'), str) and value['description'].strip(),
            'Plugin description is required')


def validate_catalog(host, value):
    plugin = {'name': 'codecallnavigator', 'source': './'}
    expected = {'name': 'codecallnavigator-local', 'owner': {'name': 'CodeCallNavigator'},
                'plugins': [plugin]}
    if host == 'codex':
        plugin['source'] = {'source': 'local', 'path': './'}
        plugin['policy'] = {'installation': 'AVAILABLE', 'authentication': 'ON_USE'}
        plugin['category'] = 'Developer Tools'
        expected.pop('owner')
    require(value == expected, 'Unexpected catalog: one self-root plugin is required')


def reviewed_files(root, host, missing_skill=False):
    files = source_files(root, host, missing_skill)
    manifest, catalog = METADATA[host]
    validate_manifest(host, json.loads(files[manifest]))
    validate_catalog(host, json.loads(files[catalog]))
    return files


def validate(root, host):
    files = reviewed_files(root, host)
    require(files[SKILL] == wrapper(root), f'Canonical skill drift in {host}; use --sync explicitly')
    return files


def synchronize(root, host):
    reviewed_files(root, host, missing_skill=True)
    path = root / 'integrations' / host / SKILL
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(wrapper(root))


def write_archive(files, destination, name):
    plugin = destination / name
    plugin.mkdir()
    with zipfile.ZipFile(destination / f'{name}.zip', 'w', compression=zipfile.ZIP_DEFLATED) as archive:
        for relative, data in sorted(files.items()):
            path = plugin / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            path.chmod(0o644)
            entry = zipfile.ZipInfo(f'{name}/{relative}', date_time=(1980, 1, 1, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = 0o100644 << 16
            entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, data)


def build(root, hosts, output):
    require(not output.exists() and not output.is_symlink(), f'Output already exists: {output}')
    sources = {host: validate(root, host) for host in hosts}
    license_path = root / 'LICENSE'
    require(license_path.is_file() and not license_path.is_symlink(), 'LICENSE must be a regular file')
    license_bytes = license_path.read_bytes()
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.agent-integrations-', dir=output.parent) as temporary:
        staging = Path(temporary)
        for host, files in sources.items():
            write_archive(files | {'LICENSE': license_bytes}, staging, f'codecallnavigator-{host}')
        require(not output.exists() and not output.is_symlink(), f'Output already exists: {output}')
        staging.rename(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--host', choices=[*HOSTS, 'all'], default='all')
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument('--check', action='store_true', help='Read-only source/drift validation')
    modes.add_argument('--sync', action='store_true', help='Regenerate only host skill wrappers')
    parser.add_argument('--output', type=Path, help='New output root for plugin directories and ZIPs')
    args = parser.parse_args()
    hosts = list(HOSTS) if args.host == 'all' else [args.host]
    require(args.output is None if args.check or args.sync else args.output is not None,
            '--output is required only for a build')
    if args.check or args.sync:
        operation = synchronize if args.sync else validate
        for host in hosts:
            operation(ROOT, host)
    else:
        build(ROOT, hosts, args.output)
    print(f'Agent integration {"sync" if args.sync else "check" if args.check else "build"} passed: {", ".join(hosts)}')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        raise SystemExit(f'Agent integration packaging failed: {error}') from error
