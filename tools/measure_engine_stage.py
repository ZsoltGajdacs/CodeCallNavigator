#!/usr/bin/env python3
"""Measure matching CLI distributions on one project, retaining reports only.

Each run replaces the same project graph; no graph snapshots are saved. Sampled
process-tree RSS is a lower bound and can count shared pages more than once.
"""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import threading
import time

from measure_analysis import process_tree_bytes


def sample(process, stopped, observations, errors):
    while not stopped.wait(0.1):
        try:
            observations.append(process_tree_bytes(process.pid))
        except (OSError, subprocess.CalledProcessError) as error:
            errors.append(str(error))
            return


def run(executable, project, targets):
    started = time.perf_counter()
    process = subprocess.Popen([str(executable), 'analyze', '--project', str(project), *targets],
                               stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    stopped, observations, errors = threading.Event(), [], []
    sampler = threading.Thread(target=sample, args=(process, stopped, observations, errors))
    sampler.start()
    try:
        _, error = process.communicate()
        if process.returncode:
            raise RuntimeError(error)
    finally:
        stopped.set()
        sampler.join()
    graph = json.loads((project / '.codecallnavigator/graph.json').read_text())['graph']
    return {'seconds': time.perf_counter() - started,
            'sampled_peak_tree_rss_bytes': max(observations) if observations else None,
            'sampling_errors': errors,
            'nodes': len(graph['nodes']), 'edges': len(graph['edges'])}


def summarize(runs):
    samples = [item['sampled_peak_tree_rss_bytes'] for item in runs if item['sampled_peak_tree_rss_bytes'] is not None]
    return {'runs': runs, 'median_seconds': statistics.median(item['seconds'] for item in runs),
            'median_sampled_peak_tree_rss_bytes': statistics.median(samples) if samples else None,
            'resource_sampling_complete': len(samples) == len(runs) and not any(item['sampling_errors'] for item in runs)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--runs', type=int, default=3)
    parser.add_argument('targets', nargs='*')
    args = parser.parse_args()
    if args.runs < 1:
        parser.error('--runs must be positive')
    project = args.project.resolve()
    results = {'baseline': [], 'candidate': []}
    for _ in range(args.runs):
        for name in results:
            results[name].append(run(getattr(args, name).resolve(), project, args.targets))
    print(json.dumps({name: summarize(runs) for name, runs in results.items()}, indent=2))


if __name__ == '__main__':
    main()
