#!/usr/bin/env python3
"""Measure a real CLI session's structural startup, flow residency and reloads.

Uses only saved artifacts; does not analyze or publish. RSS is sampled after each
reply and can retain allocator pages after unload; retained_bytes is the CLI's
allocation estimate. Source freshness work is included in status/flow timings.
"""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time


class Session:
    def __init__(self, engine, project, version):
        self.process = subprocess.Popen([str(engine), 'query-session'], stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.project = str(project)
        self.version = version
        self.generation = None
        self.samples = []

    def query(self, name, operation):
        request = {'version': self.version, 'project': self.project, 'query': operation}
        if self.generation:
            request['expected_generation'] = self.generation
        started = time.perf_counter()
        self.process.stdin.write(json.dumps(request).encode() + b'\n')
        self.process.stdin.flush()
        raw = self.process.stdout.readline()
        elapsed = (time.perf_counter() - started) * 1000
        if not raw:
            raise RuntimeError(self.process.stderr.read().decode())
        response = json.loads(raw)
        if response.get('error'):
            raise RuntimeError(response['error'])
        self.generation = response['generation']
        rss = subprocess.check_output(['ps', '-o', 'rss=', '-p', str(self.process.pid)], text=True)
        sample = {'name': name, 'milliseconds': elapsed, 'response_bytes': len(raw),
                  'rss_bytes': int(rss.strip()) * 1024}
        if operation['kind'] == 'flow_resources':
            sample['residency'] = response['result']
        self.samples.append(sample)
        return response['result']

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
            raise RuntimeError('CLI did not stop after stdin closed')
        finally:
            self.process.stdout.close()
            self.process.stderr.close()
        if self.process.returncode:
            raise RuntimeError(f'CLI exited {self.process.returncode}')


def measure(settings):
    session = Session(settings.engine, settings.project, settings.version)
    try:
        session.query('structural_startup', {'kind': 'status'})
        if settings.version >= 2:
            session.query('initial_residency', {'kind': 'flow_resources', 'action': 'status'})
        session.query('first_flow', {'kind': 'flow', 'symbol': settings.symbol})
        for index in range(settings.repeats):
            if settings.version >= 2:
                session.query(f'unload_{index}', {'kind': 'flow_resources', 'action': 'unload'})
                session.query(f'reload_{index}', {'kind': 'flow_resources', 'action': 'load'})
            session.query(f'repeated_flow_{index}', {'kind': 'flow', 'symbol': settings.symbol})
        if settings.version >= 2:
            session.query('final_unload', {'kind': 'flow_resources', 'action': 'unload'})
        return {'generation': session.generation, 'samples': session.samples}
    finally:
        session.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--engine', type=Path, required=True)
    parser.add_argument('--project', type=Path, required=True)
    parser.add_argument('--symbol', required=True)
    parser.add_argument('--version', type=int, choices=[1, 2], default=2)
    parser.add_argument('--repeats', type=int, default=3)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.engine = args.engine.resolve()
    args.project = args.project.resolve()
    result = measure(args)
    reloads = [sample['milliseconds'] for sample in result['samples'] if sample['name'].startswith('reload_')]
    if reloads:
        result['reload_median_ms'] = statistics.median(reloads)
        result['reload_max_ms'] = max(reloads)
    result['method'] = 'Saved-artifact session; uncontrolled filesystem caches; reply-time RSS samples, not peak allocation; source freshness work included for status/flow.'
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
