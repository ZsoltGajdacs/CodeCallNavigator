"""Real child-command probes for isolated CLI acceptance-case scheduling.

These prove harness lifecycle behavior, not successful compiler analysis.
"""
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import csharp_flow_ingestion_acceptance as ingestion

PROBE = r'''
import fcntl, json, os, pathlib, sys, time
name, phase, shared, mode = sys.argv[1:]
phase = int(phase)
shared = pathlib.Path(shared)
owned = pathlib.Path.cwd() / 'mutation.json'
def record(kind):
    event = dict(name=name, phase=phase, kind=kind, time=time.monotonic_ns(),
                 work=str(pathlib.Path.cwd()), owner=os.environ.get('CASE_NAME'),
                 derived=os.environ.get('CASE_STEP'))
    with (shared / 'events').open('a') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        stream.write(json.dumps(event) + '\n')
        stream.flush()
def wait_for(predicate):
    deadline = time.monotonic() + 5
    while not predicate():
        if time.monotonic() >= deadline:
            raise RuntimeError('Case rendezvous deadline exceeded')
        time.sleep(.005)
if phase:
    assert json.loads(owned.read_text()) == dict(owner=name, phase=phase - 1)
assert os.environ['CASE_NAME'] == name
assert os.environ.get('CASE_STEP') == ('derived' if phase == 1 else None)
record('start')
if phase == 0:
    (shared / ('started-' + name)).touch()
    wait_for(lambda: len(list(shared.glob('started-*'))) >= 4)
    if mode == 'failure' and name == 'case0':
        (shared / 'failed').touch()
        record('end')
        sys.exit(7)
if mode == 'failure':
    wait_for(lambda: (shared / 'failed').exists())
if phase == 2 and mode == 'overlap' and name == 'case0':
    wait_for(lambda: (shared / 'done-case3').exists())
time.sleep(.025)
owned.write_text(json.dumps(dict(owner=name, phase=phase)))
record('end')
if phase == 2:
    (shared / ('done-' + name)).touch()
'''


class IngestionScheduling(unittest.TestCase):
    def fixture(self, root, mode):
        (root / 'work').mkdir()
        engine = root / 'command-probe'
        engine.write_text(f'#!{sys.executable}\n' + PROBE)
        engine.chmod(0o755)
        cases = [(f'case{index}', self.callback(f'case{index}', root, mode)) for index in range(6)]
        return engine, cases

    def callback(self, name, shared, mode):
        def run(client, work):
            client.environment['CASE_NAME'] = name
            for phase in range(3):
                current = client.in_environment({**client.environment, 'CASE_STEP':'derived'}) if phase == 1 else client
                current.command([name, str(phase), str(shared), mode], work)
            return dict(name=name, work=str(work), records=client.records)
        return run

    def events(self, root):
        return [json.loads(line) for line in (root / 'events').read_text().splitlines()]

    def event_names(self, events, phase, kind):
        return [event['name'] for event in events
                if event['phase'] == phase and event['kind'] == kind]

    def selected_commands(self, records):
        return [record['arguments'][:2] for record in records]

    def assert_budget(self, events):
        active, maximum = 0, 0
        for event in sorted(events, key=lambda event:event['time']):
            if event['phase'] == 0 and event['kind'] == 'start':
                active += 1
                maximum = max(maximum, active)
            if event['phase'] == 2 and event['kind'] == 'end':
                active -= 1
        self.assertEqual(active, 0)
        self.assertEqual(maximum, 4)

    def assert_isolated_cases(self, outcomes, environment, original, work):
        self.assertEqual(environment, original)
        self.assertEqual(list(outcomes), [f'case{index}' for index in range(6)])
        self.assertEqual(len({value['work'] for value in outcomes.values()}), 6)
        self.assertEqual(len({id(value['records']) for value in outcomes.values()}), 6)
        for name, outcome in outcomes.items():
            self.assertTrue(Path(outcome['work']).is_relative_to(work))
            self.assertEqual(len(outcome['records']), 3)
            self.assertEqual(self.selected_commands(outcome['records']),
                             [[name, str(phase)] for phase in range(3)])

    def test_cases_overlap_with_isolated_mutations_and_ordered_reports(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            engine, cases = self.fixture(root, 'overlap')
            environment = dict(os.environ)
            environment.pop('CASE_STEP', None)
            original = environment.copy()
            outcomes, records = ingestion.run_cases(engine, environment, cases, root / 'work')
            self.assert_isolated_cases(outcomes, environment, original, root / 'work')
            self.assertEqual(self.selected_commands(records),
                             [[f'case{index}', str(phase)] for index in range(6) for phase in range(3)])
            events = self.events(root)
            self.assert_budget(events)
            completed = self.event_names(events, 2, 'end')
            self.assertLess(completed.index('case3'), completed.index('case0'))

    def test_failure_prevents_pending_cases_and_drains_active_cases(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            engine, cases = self.fixture(root, 'failure')
            environment = dict(os.environ)
            environment.pop('CASE_STEP', None)
            with self.assertRaisesRegex(ValueError, 'returned 7'):
                ingestion.run_cases(engine, environment, cases, root / 'work')
            events = self.events(root)
            started = self.event_names(events, 0, 'start')
            finished = self.event_names(events, 2, 'end')
            self.assertCountEqual(started, [f'case{index}' for index in range(4)])
            self.assertCountEqual(finished, ['case1', 'case2', 'case3'])
        self.assertFalse(root.exists())


if __name__ == '__main__':
    unittest.main()
