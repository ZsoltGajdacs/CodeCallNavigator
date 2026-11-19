"""Independent v2 bundle reader and disposable verification-fixture publisher.

Application storage and publication remain owned by the Rust CLI. This module
lets schema and acceptance checks inspect or deliberately mutate saved fixtures.
"""
import copy
from contextlib import closing, contextmanager
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import subprocess
import tempfile

import blake3
import zstandard

SCHEMA = 'codecallnavigator.graph.manifest.v2'


def graph_path(project, engine, environment=None, *, required=True):
    """Ask the production engine for the manifest of the current generation."""
    result = subprocess.run([str(engine), "graph-path", "--project", str(project)],
                            env=environment, capture_output=True, text=True, timeout=30)
    if result.returncode:
        if not required and "No saved working graph in the local library" in result.stderr:
            return None
        raise RuntimeError(f"Cannot locate the graph for {project}: {result.stderr.strip()}")
    return Path(result.stdout.strip())


def object_root(path):
    path = Path(path)
    if (path.parent / 'objects').is_dir():
        return path.parent
    return next((parent for parent in path.parents if (parent / 'catalog.sqlite3').is_file()),
                path.parent)


@contextmanager
def fixture_database(root):
    """Operate on a temporary library under the engine's publication guard."""
    import fcntl

    root = root.resolve()
    if not root.is_relative_to(Path(tempfile.gettempdir()).resolve()):
        raise ValueError("Fixture mutation requires a temporary graph library")
    with (root / 'library.lock').open('rb') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        with closing(sqlite3.connect(root / 'catalog.sqlite3')) as connection, connection:
            yield connection


def fixture_state_dir(path, project):
    root = object_root(path)
    key = blake3.blake3(os.fsencode(Path(project).resolve())).hexdigest()
    return root / 'state' / key


def working_entry(connection, path):
    bundle = connection.execute(
        "SELECT entry FROM bundles JOIN working ON bundles.id=working.current WHERE bundles.id=?",
        (path.parent.name,)).fetchone()
    if bundle is None:
        raise ValueError("Fixture path must identify the current working graph")
    entry = json.loads(bundle[0])
    if "Working" not in entry["location"]:
        raise ValueError("Fixture mutation requires a working-only graph")
    return entry


def reference_fixture(connection, entry, root):
    location = entry['location']['Working']
    manifest = entry['manifest'].encode()
    identifier = blake3.blake3(encoded([location['checkout'], location['project'],
        blake3.blake3(manifest).hexdigest(), entry['metadata']])).hexdigest()
    path = root / 'worktrees' / location['checkout'] / location['project'] / identifier / 'graph.json'
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix('.fixture-tmp')
    temporary.write_bytes(manifest)
    temporary.replace(path)
    connection.execute("INSERT OR IGNORE INTO bundles (id, entry) VALUES (?, ?)",
                       (identifier, json.dumps(entry)))
    project = location['project']
    previous, = connection.execute("SELECT previous FROM working WHERE project=?", (project,)).fetchone()
    binding = blake3.blake3(encoded([project, identifier, previous])).hexdigest()
    connection.execute("UPDATE working SET current=?, binding=? WHERE project=?",
                       (identifier, binding, project))
    return path


def encoded(value):
    return json.dumps(value, separators=(',', ':'), sort_keys=True, ensure_ascii=False).encode()


def part_path(directory, part, kind):
    digest = part['digest']
    if not re.fullmatch(r'[0-9a-f]{64}', digest):
        raise ValueError('Invalid object digest')
    return directory / 'objects' / f'{digest}.{kind}.json.zst'


def copy_bundle(path, destination):
    """Copy exactly one graph and its contracts into a portable fixture."""
    root = object_root(path)
    raw = path.read_bytes()
    manifest = json.loads(raw)
    objects = destination / 'objects'
    objects.mkdir(parents=True, exist_ok=True)
    for kind in ('structure', 'flows'):
        source = part_path(root, manifest[kind], kind)
        shutil.copyfile(source, objects / source.name)
    for name in ('graph.schema.json', 'graph.manifest.schema.json', 'README.md'):
        shutil.copyfile(root / name, destination / name)
    (destination / 'graph.json').write_bytes(raw)


def part_bytes(directory, part, kind):
    compressed = part_path(directory, part, kind).read_bytes()
    if len(compressed) != part['compressed_bytes'] or blake3.blake3(compressed).hexdigest() != part['digest']:
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
    root = object_root(path)
    artifact = json.loads(part_bytes(root, manifest['structure'], 'structure'))
    artifact['graph'].pop('flow_catalogue', None)
    artifact['graph']['flow_inventory'] = json.loads(part_bytes(root, manifest['flows'], 'flows'))
    return artifact


def semantic_bytes(path):
    return encoded(load(path)) + b'\n'


def stored_bytes(path):
    path = Path(path)
    manifest = json.loads(path.read_bytes())
    return path.stat().st_size + sum(manifest[kind]['compressed_bytes'] for kind in ('structure', 'flows'))


def encode_fixture(artifact):
    structure = copy.deepcopy(artifact)
    flows = structure['graph'].pop('flow_inventory', None)
    structure['graph']['flow_catalogue'] = {
        'inventory_version': flows['version'] if flows else None,
        'symbols': sorted({function['symbol'] for function in flows['functions']}) if flows else [],
    }
    manifest = {'schema': SCHEMA}
    objects = {}
    for kind, value in [('structure', structure), ('flows', flows)]:
        raw = encoded(value)
        compressed = zstandard.ZstdCompressor(level=9).compress(raw)
        digest = blake3.blake3(compressed).hexdigest()
        objects[f'objects/{digest}.{kind}.json.zst'] = compressed
        manifest[kind] = {'digest': digest, 'compressed_bytes': len(compressed), 'uncompressed_bytes': len(raw)}
    return encoded(manifest).decode() + '\n', objects


def write_fixture_objects(root, objects):
    for name, content in objects.items():
        target = root / name
        if target.exists() and target.read_bytes() == content:
            continue
        with tempfile.NamedTemporaryFile(dir=target.parent, delete=False) as temporary:
            temporary.write(content)
        Path(temporary.name).replace(target)


def publish_fixture(path, artifact):
    """Deliberately alter a real working graph for version-compatibility fixtures."""
    manifest, objects = encode_fixture(artifact)
    root = object_root(path)
    with fixture_database(root) as connection:
        entry = working_entry(connection, path)
        entry['manifest'] = manifest
        write_fixture_objects(root, objects)
        return reference_fixture(connection, entry, root)


def capture(path):
    """Retain a complete disposable baseline across production object pruning."""
    path = Path(path)
    root = object_root(path)
    with fixture_database(root) as connection:
        entry = working_entry(connection, path)
        manifest = json.loads(path.read_bytes())
        names = (f"objects/{manifest[kind]['digest']}.{kind}.json.zst"
                 for kind in ('structure', 'flows'))
        objects = {name: (root / name).read_bytes() for name in names}
    return entry, objects


def restore(path, snapshot):
    root = object_root(path)
    entry, objects = snapshot
    with fixture_database(root) as connection:
        write_fixture_objects(root, objects)
        return reference_fixture(connection, entry, root)
