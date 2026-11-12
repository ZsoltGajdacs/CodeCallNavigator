#!/usr/bin/env python3
"""Exercise companion workflow protocol facts against the real compiler-backed public CLI."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from agent_cli import AcceptanceError, Client, require


def fixture(root, modules):
    (root / 'node_modules').symlink_to(modules, target_is_directory=True)
    (root / 'tsconfig.json').write_text(json.dumps({
        'compilerOptions': {'strict': True, 'target': 'ES2022', 'types': []},
        'include': ['*.ts'],
    }))
    (root / 'entry.ts').write_text(
        'export function leaf(): number { return 1; }\n'
        'export function start(flag: boolean): number { if(flag) return leaf(); return 0; }\n'
        'export interface Contract { run(): void; }\n')
    (root / 'other.ts').write_text('export function start(): number { return 2; }\n')


def methods(client, text, generation):
    response = client.query('resolve-' + text, {
        'kind': 'search', 'text': text, 'category': 'methods', 'offset': 0, 'limit': 20,
    }, generation)
    return response['result']['hits']


def choose_entry(client, hits, generation):
    candidates = []
    for hit in hits:
        symbol = hit['target']['symbol']
        details = client.query('disambiguate', {'kind': 'symbol', 'symbol': symbol}, generation)
        if details['result']['file'].endswith('/entry.ts'):
            candidates.append(symbol)
    require(len(candidates) == 1, 'Expected exactly one entry.ts method candidate')
    return candidates[0]


def section(client, symbol, direction, generation):
    return client.query('section-' + direction, {
        'kind': 'selection',
        'selection': {'kind': 'neighborhood', 'symbols': [symbol],
                      'direction': direction, 'depth': 1},
        'limits': {'nodes': 100, 'edges': 300},
    }, generation)['result']


def inspect_graph(client, generation):
    hits = methods(client, 'start', generation)
    require(len(hits) == 2, 'Same-named start methods must remain ambiguous')
    symbol = choose_entry(client, hits, generation)
    outgoing = section(client, symbol, 'Outgoing', generation)
    incoming = section(client, symbol, 'Incoming', generation)
    require(len(outgoing['edges']) == 1, 'entry.ts start must have its leaf callee')
    require(not incoming['edges'], 'entry.ts start must not gain callers')
    bounded = client.query('bounded-section', {
        'kind': 'selection',
        'selection': {'kind': 'neighborhood', 'symbols': [symbol],
                      'direction': 'Outgoing', 'depth': 1},
        'limits': {'nodes': 1, 'edges': 1},
    }, generation)['result']
    require(bounded['truncated'] and bounded['omitted_nodes'] > 0,
            'Tight graph limits must report omitted nodes')
    flow = client.query('function-flow', {'kind': 'flow', 'symbol': symbol}, generation)
    require(flow['result']['chart']['steps'], 'Authored start must have source flow')
    unsupported = methods(client, 'run', generation)
    require(len(unsupported) == 1, 'Expected one body-less run declaration')
    client.query('unsupported-flow', {
        'kind': 'flow', 'symbol': unsupported[0]['target']['symbol'],
    }, generation, error='flow_unavailable')
    return symbol


def edit_and_refresh(client, root, generation, symbol):
    artifact = root / '.codecallnavigator/graph.json'
    before = artifact.read_bytes()
    (root / 'other.ts').write_text('export function start(): number { return 42; }\n')
    status = client.query('edited-inputs', {'kind': 'status'})
    require(status['freshness']['input_status'] == 'stale', 'Edited inputs must become stale')
    client.query('analyzed-source', {
        'kind': 'source', 'symbol': symbol, 'origin': 'analyzed',
    }, generation)
    repeated = client.query('repeated-status', {'kind': 'status'}, generation)
    require(repeated['generation'] == generation, 'Queries must not replace the generation')
    require(artifact.read_bytes() == before, 'Read queries must preserve artifact bytes')
    client.analyze('explicit-refresh-after-edit')
    client.query('old-generation', {'kind': 'status'}, generation, error='generation_mismatch')
    latest = client.query('new-generation', {'kind': 'status'})
    require(latest['generation'] != generation, 'Explicit refresh must publish a new generation')


def protocol(client, root):
    capabilities = json.loads(client.command('capabilities', ['capabilities']))
    require(capabilities.get('query_version') == 1 and capabilities.get('generation_binding'),
            'CLI must support query contract version 1 and generation binding')
    client.query('missing-graph', {'kind': 'status'}, error='graph_missing')
    require(not (root / '.codecallnavigator/graph.json').exists(),
            'Status must not create a missing graph')
    client.command('discover', ['discover', str(root)])
    client.analyze('explicit-initial-refresh')
    status = client.query('initial-status', {'kind': 'status'})
    freshness = status['freshness']['input_status']
    require(freshness in {'match', 'unverifiable'}, 'Initial graph cannot be stale')
    if freshness == 'unverifiable':
        require(status['freshness']['reasons'], 'Unverifiable freshness must retain reasons')
    symbol = inspect_graph(client, status['generation'])
    edit_and_refresh(client, root, status['generation'], symbol)
    return freshness


def unavailable(root, environment):
    try:
        subprocess.run([str(root / 'missing-ccnavigator'), 'capabilities'],
                       env=environment, check=True, timeout=5)
    except FileNotFoundError:
        return 'executable-not-found'
    raise AcceptanceError('Missing CLI executable unexpectedly ran')


def run(args):
    with tempfile.TemporaryDirectory(prefix='agent-journeys-') as directory:
        root = Path(directory).resolve()
        fixture(root, args.node_modules)
        environment = os.environ | {
            'CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT': str(args.node_modules / 'typescript'),
        }
        client = Client(args.engine, root, environment)
        freshness = protocol(client, root)
        report = {'status': 'passed', 'initial_freshness': freshness,
                  'host_behavior': 'not-tested; see integrations/shared/journeys.md',
                  'records': client.records, 'unavailable_cli': unavailable(root, environment)}
        serialized = json.dumps(report, indent=2).replace(str(root), '<temporary-project>')
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized + '\n')
        print(f'Passed {len(client.records)} real CLI operations; report: {args.output}')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', default='ccnavigator',
                        help='CLI command on PATH, or an explicit executable path')
    for name in ['node-modules', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    executable = shutil.which(args.engine)
    require(executable, f'Missing required engine executable: {args.engine}')
    args.engine = Path(executable).resolve()
    args.node_modules = args.node_modules.resolve()
    require(args.node_modules.is_dir(), f'Missing node_modules: {args.node_modules}')
    return args


if __name__ == '__main__':
    try:
        run(arguments())
    except (AcceptanceError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit(f'Agent journey acceptance failed: {error}') from error
