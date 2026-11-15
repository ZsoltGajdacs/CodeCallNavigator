"""Cargo target ownership for test identities, including implicit binaries."""
import tomllib


def binary_specs(data, directory):
    yield from data.get('bin', [])
    for _ in directory.glob('src/main.rs'):
        yield dict(name=data['package']['name'], path='src/main.rs')
    for source in sorted((directory / 'src/bin').glob('*.rs')):
        yield dict(name=source.stem, path=str(source.relative_to(directory)))


def binary_targets(data, directory):
    binaries = {}
    for entry in binary_specs(data, directory):
        binaries.setdefault(entry['name'], entry)
    return [entry for entry in binaries.values() if entry.get('test', True)]


def library_target(data, directory):
    package = data['package']['name']
    for source in directory.glob('src/lib.rs'):
        yield source, dict(package=package, target_kind='lib', target=package.replace('-', '_'))


def binary_test_targets(data, directory):
    package = data['package']['name']
    for entry in binary_targets(data, directory):
        yield directory / entry['path'], dict(package=package, target_kind='bin', target=entry['name'])


def integration_targets(data, directory):
    package = data['package']['name']
    for source in sorted((directory / 'tests').glob('*.rs')):
        yield source, dict(package=package, target_kind='test', target=source.stem)


def package_targets(manifest):
    data = tomllib.loads(manifest.read_text())
    directory = manifest.parent
    yield from library_target(data, directory)
    yield from binary_test_targets(data, directory)
    yield from integration_targets(data, directory)


def rust_targets(root):
    manifests = [root / 'Cargo.toml', *sorted((root / 'crates').glob('*/Cargo.toml'))]
    for manifest in manifests:
        yield from package_targets(manifest)
