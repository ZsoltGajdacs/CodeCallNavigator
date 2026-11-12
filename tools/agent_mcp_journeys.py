#!/usr/bin/env python3
"""Exercise native SDK MCP tools with a real compiler fixture and PATH-installed CLI."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from agent_cli import AcceptanceError, require
from agent_journeys import fixture
from agent_mcp import Client


def methods(client, text, generation):
    return client.call('resolve-' + text, 'find_symbols', {
        'text': text, 'category': 'methods', 'offset': 0, 'limit': 20,
        'expected_generation': generation,
    })['result']['hits']


def choose_entry(client, hits, generation):
    candidates = []
    for hit in hits:
        symbol = hit['target']['symbol']
        details = client.call('disambiguate', 'symbol_details', {
            'symbol': symbol, 'expected_generation': generation,
        })
        if details['result']['file'].endswith('/entry.ts'):
            candidates.append(symbol)
    require(len(candidates) == 1, 'Expected exactly one entry.ts method candidate')
    return candidates[0]


def section(client, symbol, direction, generation):
    return client.call('section-' + direction, 'graph_section', {
        'selection': {'kind': 'neighborhood', 'symbols': [symbol],
                      'direction': direction, 'depth': 1},
        'limits': {'nodes': 100, 'edges': 300}, 'expected_generation': generation,
    })['result']


def inspect_graph(client, generation):
    hits = methods(client, 'start', generation)
    require(len(hits) == 2, 'Same-named start methods must remain ambiguous')
    symbol = choose_entry(client, hits, generation)
    outgoing = section(client, symbol, 'Outgoing', generation)
    incoming = section(client, symbol, 'Incoming', generation)
    require(len(outgoing['edges']) == 1, 'entry.ts start must have its leaf callee')
    require(not incoming['edges'], 'entry.ts start must not gain callers')
    bounded = client.call('bounded-section', 'graph_section', {
        'selection': {'kind': 'neighborhood', 'symbols': [symbol],
                      'direction': 'Outgoing', 'depth': 1},
        'limits': {'nodes': 1, 'edges': 1}, 'expected_generation': generation,
    })['result']
    require(bounded['truncated'] and bounded['omitted_nodes'] > 0,
            'Tight graph limits must report omitted nodes')
    flow = client.call('function-flow', 'function_flow', {
        'symbol': symbol, 'expected_generation': generation,
    })
    require(flow['result']['chart']['steps'], 'Authored start must have source flow')
    unsupported = methods(client, 'run', generation)
    require(len(unsupported) == 1, 'Expected one body-less run declaration')
    client.call('unsupported-flow', 'function_flow', {
        'symbol': unsupported[0]['target']['symbol'], 'expected_generation': generation,
    }, error='flow_unavailable')
    return symbol


def edit_and_refresh(client, root, generation, symbol):
    artifact = root / '.codecallnavigator/graph.json'
    before = artifact.read_bytes()
    (root / 'other.ts').write_text('export function start(): number { return 42; }\n')
    status = client.call('edited-inputs', 'graph_status', {})
    require(status['freshness']['input_status'] == 'stale', 'Edited inputs must become stale')
    client.call('analyzed-source', 'symbol_source', {
        'symbol': symbol, 'origin': 'analyzed', 'expected_generation': generation,
    })
    repeated = client.call('repeated-status', 'graph_status', {'expected_generation': generation})
    require(repeated['generation'] == generation, 'Queries must not replace the generation')
    require(artifact.read_bytes() == before, 'Read queries must preserve artifact bytes')
    client.call('explicit-refresh-after-edit', 'refresh_graph', {})
    client.call('old-generation', 'graph_status', {'expected_generation': generation},
                error='generation_mismatch')
    latest = client.call('new-generation', 'graph_status', {})
    require(latest['generation'] != generation, 'Explicit refresh must publish a new generation')


def protocol(client, root):
    tools = client.request('tools/list', {})
    names = {tool['name'] for tool in tools['tools']}
    require({'graph_status', 'find_symbols', 'graph_section', 'function_flow', 'refresh_graph'} <= names,
            'MCP server must advertise companion workflow tools')
    client.call('missing-graph', 'graph_status', {}, error='graph_missing')
    require(not (root / '.codecallnavigator/graph.json').exists(),
            'Status must not create a missing graph')
    client.call('discover', 'discover_builds', {})
    client.call('explicit-initial-refresh', 'refresh_graph', {})
    status = client.call('initial-status', 'graph_status', {})
    freshness = status['freshness']['input_status']
    require(freshness in {'match', 'unverifiable'}, 'Initial graph cannot be stale')
    if freshness == 'unverifiable':
        require(status['freshness']['reasons'], 'Unverifiable freshness must retain reasons')
    symbol = inspect_graph(client, status['generation'])
    edit_and_refresh(client, root, status['generation'], symbol)
    return freshness


def unavailable(args, root, environment):
    command = [args.mcp, '--engine', str(root / 'missing-ccnavigator'), '--project', str(root)]
    result = subprocess.run(command, env=environment, capture_output=True, timeout=20)
    require(result.returncode != 0, 'Missing CLI must fail MCP startup')
    try:
        subprocess.run([str(root / 'missing-mcp')], check=True, timeout=5)
    except FileNotFoundError:
        return {'cli_exit': result.returncode, 'mcp': 'executable-not-found'}
    raise AcceptanceError('Missing MCP executable unexpectedly ran')


def run(args):
    with tempfile.TemporaryDirectory(prefix='agent-mcp-journeys-') as directory:
        root = Path(directory).resolve()
        fixture(root, args.node_modules)
        environment = os.environ | {
            'CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT': str(args.node_modules / 'typescript'),
        }
        command = [args.mcp, '--engine', args.engine, '--project', str(root)]
        with Client(command, environment) as client:
            freshness = protocol(client, root)
        report = {'status': 'passed', 'initial_freshness': freshness,
                  'host_behavior': 'not-tested; see integrations/shared/journeys.md',
                  'records': client.records, 'unavailable': unavailable(args, root, environment)}
        serialized = json.dumps(report, indent=2).replace(str(root), '<temporary-project>')
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(serialized + '\n')
        print(f'Passed {len(client.records)} real MCP tool operations; report: {args.output}')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', default='ccnavigator', help='Installed CLI command on PATH')
    parser.add_argument('--mcp', default='codecallnavigator-mcp', help='Installed MCP command on PATH')
    parser.add_argument('--node-modules', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    for name in ['engine', 'mcp']:
        require(shutil.which(getattr(args, name)), f'Missing required {name} executable')
    args.node_modules = args.node_modules.resolve()
    require(args.node_modules.is_dir(), f'Missing node_modules: {args.node_modules}')
    return args


if __name__ == '__main__':
    try:
        run(arguments())
    except (AcceptanceError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit(f'Native MCP journey acceptance failed: {error}') from error
