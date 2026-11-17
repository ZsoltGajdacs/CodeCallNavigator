"""Real libtest-protocol probes for bounded verification scheduling."""
import contextlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

# Narrow libtest protocol probes exercise real runner scheduling and child draining.
# They do not simulate compiler decisions or certify analyzer behavior.
SCHEDULER_PROBE = r'''
import fcntl, json, os, pathlib, sys, time
root = pathlib.Path(config['root'])
if '--list' in sys.argv:
    print('\n'.join(name + ': test' for name in config['names']))
    sys.exit(0)
names = [arg for arg in sys.argv[1:] if arg in config['names']]
def record(kind, name):
    with (root / 'events').open('a') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        stream.write(json.dumps({'kind':kind, 'name':name, 'time':time.monotonic_ns(),
                                'args':sys.argv[1:], 'flags':os.environ.get('CCNAV_TEST_FLAGS')}) + '\n')
        stream.flush()
def wait_for(predicate):
    end = time.monotonic() + 5
    while not predicate():
        if time.monotonic() >= end:
            raise RuntimeError('Probe synchronization deadline exceeded')
        time.sleep(.005)
for name in names:
    print('START ' + name, flush=True)
    record('start', name)
    (root / ('started-' + name)).touch()
    if config['mode'] in ('overlap', 'failure'):
        wait_for(lambda: len(list(root.glob('started-*'))) >= 4)
    if config['mode'] == 'serial' and name in ('case0', 'case2'):
        peer = 'case2' if name == 'case0' else 'case0'
        wait_for(lambda: (root / ('started-' + peer)).exists())
    if config['mode'] == 'failure':
        if name == 'case0':
            (root / 'failure').touch()
            record('end', name)
            sys.exit(7)
        wait_for(lambda: (root / 'failure').exists())
    time.sleep(.15)
    record('end', name)
    print('END ' + name, flush=True)
'''


class RustScheduling(unittest.TestCase):
    def probe(self, root, specs, mode='normal'):
        from verification import suite_execution
        rows, artifacts = [], []
        targets = {}
        for index, spec in enumerate(specs):
            row = dict(id=f'probe:{index}', runner='rust', package='codecallnavigator',
                       target_kind='test', target=spec.get('target', f'bin{index}'),
                       test_name=f'case{index}', category='integration', requirements=[],
                       environment_append={'CCNAV_TEST_FLAGS':f'append{index}'})
            row.update(spec)
            rows.append(row)
            targets.setdefault(row['target'], []).append(row['test_name'])
        for target, names in targets.items():
            executable = root / target
            config = dict(root=str(root), names=names, mode=mode)
            executable.write_text(f'#!{sys.executable}\nconfig = {config!r}\n' + SCHEDULER_PROBE)
            executable.chmod(0o755)
            artifacts.append(dict(executable=str(executable), package_id=root.as_uri() + '#probe',
                                  profile={'test':True}, target={'kind':['test'], 'name':target}))
        (root / 'rust-artifacts.json').write_text(json.dumps(artifacts))
        return suite_execution, rows

    def events(self, root):
        return [json.loads(line) for line in (root / 'events').read_text().splitlines()]

    def intervals(self, events):
        return {event['name']: (event['time'], next(end['time'] for end in events
                if end['kind'] == 'end' and end['name'] == event['name']))
                for event in events if event['kind'] == 'start'}

    def assert_ordered_logs(self, output, rows):
        markers = [line for line in output.getvalue().splitlines()
                   if line.startswith(('START ', 'END '))]
        expected = [marker + row['test_name'] for row in rows
                    for marker in ('START ', 'END ')]
        self.assertEqual(markers, expected)

    def test_independent_groups_overlap_with_one_global_four_worker_budget(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            runner, rows = self.probe(root, [{} for _ in range(6)], 'overlap')
            rows[1]['ignored'] = True
            output = io.StringIO()
            with patch.dict(os.environ, CCNAV_TEST_FLAGS='base'), contextlib.redirect_stdout(output):
                runner.rust_run(root, root, rows, rows)
            self.assert_ordered_logs(output, rows)
            events = self.events(root)
            starts = [event for event in events if event['kind'] == 'start']
            self.assertCountEqual([event['name'] for event in starts], [row['test_name'] for row in rows])
            active, maximum = 0, 0
            for event in sorted(events, key=lambda event:event['time']):
                active += 1 if event['kind'] == 'start' else -1
                maximum = max(maximum, active)
            self.assertEqual(active, 0)
            self.assertEqual(maximum, 4)
            for event in starts:
                index = int(event['name'].removeprefix('case'))
                self.assertIn('--exact', event['args'])
                self.assertIn('--test-threads=1', event['args'])
                self.assertEqual('--ignored' in event['args'], index == 1)
                self.assertEqual(event['flags'], f'base append{index}')

    def test_same_serial_group_across_binaries_excludes_itself_only(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            runner, rows = self.probe(root, [dict(serial_group='shared'), dict(serial_group='shared'), {}, {}], 'serial')
            runner.rust_run(root, root, rows, rows)
            intervals = self.intervals(self.events(root))
            first, second = intervals['case0'], intervals['case1']
            self.assertTrue(first[1] <= second[0] or second[1] <= first[0])
            self.assertTrue(any(max(first[0], intervals[name][0]) < min(first[1], intervals[name][1])
                                for name in ('case2', 'case3')))

    def test_native_and_performance_groups_are_exclusive_even_inside_one_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            runner, rows = self.probe(root, [dict(target='mixed', environment_append={}),
                                           dict(target='mixed', requirements=['native'], environment_append={}),
                                           dict(category='performance'), {}, {}])
            runner.rust_run(root, root, rows, rows)
            intervals = self.intervals(self.events(root))
            for exclusive in ('case1', 'case2'):
                first = intervals[exclusive]
                for name, second in intervals.items():
                    if name != exclusive:
                        self.assertTrue(first[1] <= second[0] or second[1] <= first[0], (exclusive, name))

    def test_failure_stops_pending_groups_and_drains_started_children(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            runner, rows = self.probe(root, [{} for _ in range(8)], 'failure')
            with self.assertRaises(subprocess.CalledProcessError) as failure:
                runner.rust_run(root, root, rows, rows)
            self.assertEqual(failure.exception.returncode, 7)
            events = self.events(root)
            self.assertCountEqual([event['name'] for event in events if event['kind'] == 'start'],
                                  [f'case{index}' for index in range(4)])
            self.assertCountEqual([event['name'] for event in events if event['kind'] == 'end'],
                                  [f'case{index}' for index in range(4)])

    def test_missing_selected_executable_fails_before_any_test_is_started(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            runner, rows = self.probe(root, [{}])
            missing = dict(rows[0], id='missing', target='absent', test_name='absent')
            with self.assertRaisesRegex(ValueError, 'no executable'):
                runner.rust_run(root, root, [*rows, missing], [*rows, missing])
            self.assertFalse((root / 'events').exists())


if __name__ == "__main__":
    unittest.main()
