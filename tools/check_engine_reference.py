#!/usr/bin/env python3
"""Compare complete native analysis against a frozen pre-extraction executable.

Fixtures are copied once and both engines see those exact paths. All graph data
is compared in memory; no graph histories are written to analyzed projects.
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile

from analysis_graph import canonical, graph_snapshot

ROOT = Path(__file__).resolve().parents[1]


def copy_fixture(source, target):
    shutil.copytree(source, target, ignore=shutil.ignore_patterns(
        'target', 'build', 'bin', 'obj', '.gradle', 'node_modules', '.codegraph'))


def prepare(workload, directory):
    copy_fixture(ROOT / workload['source'], directory)
    if wrapper := workload.get('wrapper'):
        (directory / 'gradlew').write_text(
            '#!/bin/sh\nexec ' + shlex.quote(str(ROOT / wrapper)) + ' "$@"\n')
        (directory / 'gradlew').chmod(0o755)
    if workload.get('dependencies'):
        modules = Path(os.environ.get('CODEGRAPH_TS_NODE_MODULES',
                       ROOT.parent / 'PacificFateMapEditor/node_modules')).resolve()
        (directory / 'node_modules').symlink_to(modules, target_is_directory=True)
    return [str((directory / target).resolve()) for target in workload['targets']]


def from_native(graph):
    identities = [node['id'] for node in graph['nodes']]
    for node in graph['nodes']:
        node.setdefault('visibility', None)
        node.setdefault('container_visibility', None)
    graph['nodes'].sort(key=lambda node: node['id'])
    for edge in graph['edges']:
        edge['from'] = identities[edge['from']]
        edge['to'] = identities[edge['to']]
    graph['edges'].sort(key=canonical)
    graph['fingerprints'] = sorted(({
        'path': item['path'], 'size': item['size'], 'hash': item['hash'],
        'missing': item.get('missing', False),
        'excluded_children': item.get('excluded_children', [])
    } for item in graph['fingerprints']), key=canonical)
    graph.pop('method_policy', None)
    return graph


def reference(executable, targets, cache):
    subprocess.run([str(executable), 'analyze', *targets], check=True,
                   env={**os.environ, 'CODEGRAPH_DATA_DIR': str(cache)},
                   stdout=subprocess.DEVNULL)
    snapshot = graph_snapshot(cache)
    snapshot.pop('method_policy')
    return snapshot


def candidate(executable, selected):
    result = subprocess.run([str(executable.resolve()), 'analyze-native', *selected],
                            check=True, capture_output=True, text=True)
    return from_native(json.loads(result.stdout))


def check_workload(args, workload, targets, root):
    name = workload['name']
    selected = targets.get(name) or [target for component in workload['components']
                                    for target in targets[component]]
    old = reference(args.reference.resolve(), selected, root / f'{name}-reference')
    new = (candidate(args.candidate, selected) if args.candidate else
           reference(args.reference.resolve(), selected, root / f'{name}-repeat'))
    differences = [key for key in old if old[key] != new.get(key)]
    print(json.dumps({'workload': name, 'equal': not differences,
                      'different_fields': differences, 'nodes': len(new['nodes']),
                      'edges': len(new['edges'])}), flush=True)
    return bool(differences)


def main():
    corpus = json.loads((ROOT / 'evidence/engine/corpus.json').read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference', type=Path, required=True)
    parser.add_argument('--candidate', type=Path)
    parser.add_argument('--workload', action='append',
                        choices=[item['name'] for item in corpus['workloads']])
    args = parser.parse_args()
    selected = [item for item in corpus['workloads']
                if not args.workload or item['name'] in args.workload]
    required = {name for item in selected
                for name in item.get('components', [item['name']])}
    with tempfile.TemporaryDirectory(prefix='codegraph-reference-') as temporary:
        root = Path(temporary)
        targets = {item['name']: prepare(item, root / item['name'])
                   for item in corpus['workloads'] if item['name'] in required}
        results = [check_workload(args, item, targets, root) for item in selected]
        return int(any(results))


if __name__ == '__main__':
    raise SystemExit(main())
