#!/usr/bin/env python3
"""Verify real-language canonical output across differently named checkouts."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from check_engine_reference import ROOT, prepare
from corpus_boundaries import configure


def analyze(executable, project, targets):
    subprocess.run([str(executable), 'analyze', '--project', str(project), *targets],
                   check=True, stdout=subprocess.DEVNULL)
    return (project / '.codecallnavigator/graph.json').read_bytes()


def targets_for(workload, workloads, project):
    if 'components' not in workload:
        return prepare(workload, project)
    project.mkdir()
    targets = [target for name in workload['components']
               for target in prepare(workloads[name], project / name)]
    configure(workload, workloads, project,
              {name: project / name for name in workload['components']})
    return targets


def check(executable, workload, workloads, directory):
    first = directory / 'first-checkout'
    second = directory / 'relocated-checkout'
    before_targets = targets_for(workload, workloads, first)
    after_targets = targets_for(workload, workloads, second)
    before = analyze(executable, first, before_targets)
    repeated = analyze(executable, first, before_targets)
    after = analyze(executable, second, list(reversed(after_targets)))
    equal = before == repeated == after
    print(json.dumps({'workload': workload['name'], 'equal': equal,
                      'repeat_equal': before == repeated, 'relocation_equal': before == after,
                      'bytes': len(before)}), flush=True)
    if not equal:
        old, new = json.loads(before)['graph'], json.loads(after)['graph']
        print('Changed sections: ' + ', '.join(key for key in old if old[key] != new.get(key)), flush=True)
    return equal


def main():
    corpus = json.loads((ROOT / 'evidence/engine/corpus.json').read_text())
    workloads = {item['name']: item for item in corpus['workloads']}
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--workload', action='append', choices=list(workloads))
    args = parser.parse_args()
    results = []
    with tempfile.TemporaryDirectory(prefix='codecallnavigator-artifact-') as temporary:
        for name in args.workload or workloads:
            directory = Path(temporary) / name
            directory.mkdir()
            results.append(check(args.executable.resolve(), workloads[name], workloads, directory))
    return int(not all(results))


if __name__ == '__main__':
    raise SystemExit(main())
