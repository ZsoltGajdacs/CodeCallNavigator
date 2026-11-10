#!/usr/bin/env python3
"""Update each fixture's one graph.json; Git is the baseline history.

Requires existing toolchains/dependencies. Never installs packages or commits.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def typescript_dependencies():
    fixture = ROOT / 'fixtures/typescript-corpus'
    modules = Path(os.environ.get('CODEGRAPH_TS_NODE_MODULES',
                   ROOT.parent / 'PacificFateMapEditor/node_modules')).resolve()
    expected = json.loads((fixture / 'package.json').read_text())['dependencies']
    for name, version in expected.items():
        actual = json.loads((modules / name / 'package.json').read_text())['version']
        if actual != version:
            raise ValueError(f'Corpus requires {name} {version}; installed {actual}')
    link = fixture / 'node_modules'
    if not link.exists():
        link.symlink_to(modules, target_is_directory=True)


def workload_targets(workload, workloads):
    if 'components' not in workload:
        return ROOT / workload['source'], workload['targets']
    targets = [str(ROOT / workloads[name]['source'] / target)
               for name in workload['components'] for target in workloads[name]['targets']]
    return ROOT / 'fixtures', targets


def main():
    corpus = json.loads((ROOT / 'evidence/engine/corpus.json').read_text())
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--workload', action='append', choices=[item['name'] for item in corpus['workloads']])
    args = parser.parse_args()
    typescript_dependencies()
    workloads = {item['name']: item for item in corpus['workloads']}
    for name in args.workload or workloads:
        workload = workloads[name]
        project, targets = workload_targets(workload, workloads)
        subprocess.run([str(args.executable.resolve()), 'analyze', '--project', str(project), *targets], check=True)


if __name__ == '__main__':
    main()
