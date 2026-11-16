"""Independent v2 bundle reader and disposable verification-fixture publisher.

Application storage and publication remain owned by the Rust CLI. This module
lets schema and acceptance checks inspect or deliberately mutate saved fixtures.
"""
import copy
import json
from pathlib import Path
import re

import blake3
import zstandard

SCHEMA = 'codecallnavigator.graph.manifest.v2'


def encoded(value):
    return json.dumps(value, separators=(',', ':'), sort_keys=True, ensure_ascii=False).encode()


def part_bytes(directory, part, kind):
    digest = part['digest']
    if not re.fullmatch(r'[0-9a-f]{64}', digest):
        raise ValueError('Invalid object digest')
    path = directory / 'objects' / f'{digest}.{kind}.json.zst'
    compressed = path.read_bytes()
    if len(compressed) != part['compressed_bytes'] or blake3.blake3(compressed).hexdigest() != digest:
        raise ValueError('Compressed object integrity mismatch')
    with zstandard.ZstdDecompressor().stream_reader(compressed) as reader:
        decoded = reader.read(part['uncompressed_bytes'] + 1)
    if len(decoded) != part['uncompressed_bytes']:
        raise ValueError('Uncompressed object length mismatch')
    return decoded


def load(path):
    path = Path(path)
    manifest = json.loads(path.read_bytes())
    if manifest.get('schema') != SCHEMA:
        raise ValueError('Unsupported saved graph format; reanalyze this project')
    artifact = json.loads(part_bytes(path.parent, manifest['structure'], 'structure'))
    artifact['graph'].pop('flow_catalogue', None)
    artifact['graph']['flow_inventory'] = json.loads(part_bytes(path.parent, manifest['flows'], 'flows'))
    return artifact


def semantic_bytes(path):
    return encoded(load(path)) + b'\n'


def stored_bytes(path):
    path = Path(path)
    manifest = json.loads(path.read_bytes())
    return path.stat().st_size + sum(manifest[kind]['compressed_bytes'] for kind in ('structure', 'flows'))


def publish_fixture(path, artifact):
    """Publish a disposable test fixture; never use for application publication."""
    path = Path(path)
    structure = copy.deepcopy(artifact)
    flows = structure['graph'].pop('flow_inventory', None)
    structure['graph']['flow_catalogue'] = {
        'inventory_version': flows['version'] if flows else None,
        'symbols': sorted({function['symbol'] for function in flows['functions']}) if flows else [],
    }
    manifest = {'schema': SCHEMA}
    directory = path.parent / 'objects'
    directory.mkdir(parents=True, exist_ok=True)
    for kind, value in [('structure', structure), ('flows', flows)]:
        raw = encoded(value)
        compressed = zstandard.ZstdCompressor(level=9).compress(raw)
        digest = blake3.blake3(compressed).hexdigest()
        (directory / f'{digest}.{kind}.json.zst').write_bytes(compressed)
        manifest[kind] = {'digest': digest, 'compressed_bytes': len(compressed), 'uncompressed_bytes': len(raw)}
    temporary = path.with_suffix('.fixture-tmp')
    temporary.write_bytes(encoded(manifest) + b'\n')
    temporary.replace(path)


def capture(path):
    """Retain a complete disposable baseline across production object pruning."""
    path = Path(path)
    raw = path.read_bytes()
    manifest = json.loads(raw)
    objects = {f"objects/{manifest[kind]['digest']}.{kind}.json.zst":
               (path.parent / f"objects/{manifest[kind]['digest']}.{kind}.json.zst").read_bytes()
               for kind in ('structure', 'flows')}
    return raw, objects


def restore(path, snapshot):
    path = Path(path)
    raw, objects = snapshot
    for name, content in objects.items():
        target = path.parent / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)
    path.write_bytes(raw)
