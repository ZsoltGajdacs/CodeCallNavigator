"""Enforce the independently installed MCP package's process boundary."""
import argparse
import json
from pathlib import Path
import re
import subprocess

FORBIDDEN = {'codecallnavigator', 'codecallnavigator-core', 'codecallnavigator-engine'}


def dependency_closure(metadata):
    packages = {package['id']: package for package in metadata['packages']}
    identity = mcp_identity(packages)
    nodes = {node['id']: node['dependencies'] for node in metadata['resolve']['nodes']}
    return [packages[key] for key in reachable(nodes, identity)]


def mcp_identity(packages):
    roots = [key for key, value in packages.items() if value['name'] == 'codecallnavigator-mcp']
    if len(roots) != 1:
        raise ValueError('Expected exactly one MCP package')
    return roots[0]


def reachable(nodes, identity):
    pending, seen = [identity], set()
    while pending:
        identity = pending.pop()
        if identity not in seen:
            seen.add(identity)
            pending.extend(nodes[identity])
    return sorted(seen)


def validate(metadata, root):
    packages = dependency_closure(metadata)
    forbidden = sorted({package['name'] for package in packages} & FORBIDDEN)
    if forbidden:
        raise ValueError('Forbidden MCP dependency, including test/target dependencies: ' + ', '.join(forbidden))
    package = next(package for package in packages if package['name'] == 'codecallnavigator-mcp')
    source_boundary(Path(package['manifest_path']).parent, root)
    return packages


def source_boundary(package, root):
    # Dependency closure catches aliases. Source checks additionally prevent hidden
    # sibling-source inclusion and filesystem access outside the CLI wire boundary.
    inclusion = r'\binclude(?:_str|_bytes)?\s*!|#\s*\[\s*path\s*='
    runtime = package / 'src'
    for path in package.rglob('*.rs'):
        source = path.read_text()
        forbidden = re.search(inclusion, source)
        if path.is_relative_to(runtime):
            forbidden = forbidden or re.search(r'\bfs\b', source)
        if forbidden:
            raise ValueError(f'MCP source boundary violation: {path.relative_to(root)}')


def inspect(root):
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--format-version', '1', '--locked', '--all-features'], cwd=root))
    packages = validate(metadata, root)
    return metadata, packages


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    _, packages = inspect(args.root.resolve())
    print(json.dumps({'boundary': 'passed', 'dependency_packages': len(packages)}))


if __name__ == '__main__':
    main()
