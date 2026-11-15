"""Owned category workspaces use real files; Usage injection probes OS uncertainty."""
import json
import os
from pathlib import Path
import select
import shutil
import subprocess
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.category_cleanup import (cleanup_category, cleanup_if_prepared,
                                           initialize_category, prepare_category, reclaim_abandoned,
                                           temporary_path)
from verification.cleanup import Usage
from verification.runner import Runner, Step
from native_fixture import NativeFixture

ROOT = Path(__file__).resolve().parents[1]


class CategoryCleanup(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.output = self.root / 'target/verification/run/unit'
        initialize_category(self.root, self.output, 'unit')
        self.track_temporary(self.output)

    def track_temporary(self, output, category='unit'):
        path = temporary_path(self.root, output)
        expected = dict(version=1, root=str(self.root), output=str(output), category=category,
                        temporary_directory=str(path))

        def remove_owned_fixture():
            if path.is_dir() and not path.is_symlink():
                marker = path / 'category-workspace.json'
                if marker.is_file() and json.loads(marker.read_text()) == expected:
                    shutil.rmtree(path)

        self.addCleanup(remove_owned_fixture)

    def payload(self, name='build'):
        path = self.output / name
        path.mkdir(exist_ok=True)
        (path / 'payload').write_bytes(b'compiled data')
        return path

    def test_owned_artifacts_are_removed_but_evidence_and_shared_caches_survive(self):
        for name in ('build', 'profile', 'test-store', 'scale-cache', 'tmp'):
            self.payload(name)
        external = temporary_path(self.root, self.output)
        self.assertLess(len(str(external).encode()), 64)
        self.assertEqual(external.stat().st_mode & 0o777, 0o700)
        (external / 'compiler-scratch').write_bytes(b'temporary compiler inputs')
        evidence = self.output / 'test.log'
        evidence.write_text('failure diagnostics')
        cache = self.root / 'target/registry'
        cache.mkdir()
        report = cleanup_category(self.root, self.output)
        self.assertEqual(report['status'], 'passed')
        self.assertGreater(report['bytes_reclaimed'], 0)
        self.assertFalse((self.output / 'build').exists())
        self.assertFalse((self.output / 'tmp').exists())
        self.assertFalse(temporary_path(self.root, self.output).exists())
        self.assertEqual(cleanup_category(self.root, self.output)['status'], 'passed')
        self.assertEqual(evidence.read_text(), 'failure diagnostics')
        self.assertTrue(cache.is_dir())
        self.assertTrue((self.output / 'results.json').exists())
        self.assert_acceptance_package_retention()
        self.assert_native_fixture_retention()

    def assert_native_fixture_retention(self):
        output = self.root / 'target/verification/native/acceptance'
        initialize_category(self.root, output, 'acceptance')
        self.track_temporary(output, 'acceptance')
        temporary = temporary_path(self.root, output)
        with NativeFixture(output / 'gate-success', 'native-success-', temporary) as success:
            (success / 'payload').write_text('successful native fixture')
        self.assertTrue(success.is_dir(), 'Category fixtures must defer removal')
        gate = output / 'gate-failed'
        with self.assertRaisesRegex(RuntimeError, 'intentional native fixture failure'):
            with NativeFixture(gate, 'native-failed-', temporary) as failed:
                (failed / 'payload').write_text('native failure diagnostics')
                raise RuntimeError('intentional native fixture failure')
        interrupted_gate = output / 'gate-interrupted'
        pending = NativeFixture(interrupted_gate, 'native-interrupted-', temporary).__enter__()
        (pending / 'payload').write_text('abruptly interrupted diagnostics')
        (pending / 'native-fixture.json.tmp').write_text('unrelated interrupted-write diagnostic')
        environment = dict(os.environ)
        environment.pop('PYTHONPATH', None)
        completed = subprocess.run([sys.executable, str(ROOT / 'tools/verification/category_cleanup.py'),
                                    '--root', str(self.root), '--output', str(output)],
                                   cwd=self.root, env=environment, capture_output=True, text=True, timeout=120)
        self.assertEqual(completed.returncode, 0, completed.stderr + completed.stdout)
        report = json.loads(completed.stdout)
        self.assertEqual(report['status'], 'passed', report)
        destination = gate / 'retained-fixture'
        self.assertEqual((destination / 'payload').read_text(), 'native failure diagnostics')
        self.assertEqual(json.loads((gate / 'retained-fixture.json').read_text())['path'], str(destination))
        self.assertGreater(report['bytes_retained'], 0)
        self.assertFalse(success.exists())
        self.assertFalse(failed.exists())
        self.assertFalse(temporary.exists())
        self.assertEqual((interrupted_gate / 'retained-fixture/payload').read_text(),
                         'abruptly interrupted diagnostics')
        self.assertEqual((interrupted_gate / 'retained-fixture/native-fixture.json.tmp').read_text(),
                         'unrelated interrupted-write diagnostic')
        self.assertEqual(json.loads((interrupted_gate / 'retained-fixture.json').read_text())['path'],
                         str(interrupted_gate / 'retained-fixture'))

    def assert_acceptance_package_retention(self):
        dist = self.root / 'dist'
        current = dist / 'CodeCallNavigator.app'
        older = dist / 'CodeCallNavigator.previous-old.app'
        newest = dist / 'CodeCallNavigator.previous-new.app'
        for path, stamp in ((current, 3000), (older, 1000), (newest, 2000)):
            path.mkdir(parents=True)
            (path / 'binary').write_bytes(b'package contents')
            os.utime(path, (stamp, stamp))
        output = self.root / 'target/verification/package/acceptance'
        initialize_category(self.root, output, 'acceptance')
        self.track_temporary(output, 'acceptance')
        report = cleanup_category(self.root, output)
        self.assertEqual(report['status'], 'passed', report)
        self.assertEqual((current / 'binary').read_bytes(), b'package contents')
        self.assertTrue(newest.is_dir())
        self.assertFalse(older.exists())
        self.assertTrue(any(row['path'] == str(older) and row['status'] == 'removed'
                            for row in report['actions']))
        receipt = (output / 'category-cleanup.json').read_bytes()
        older.mkdir()
        (older / 'binary').write_bytes(b'retained diagnostics')
        retained = self.root / 'target/verification/package-kept/acceptance'
        initialize_category(self.root, retained, 'acceptance')
        self.track_temporary(retained, 'acceptance')
        report = cleanup_category(self.root, retained, keep_artifacts=True)
        self.assertEqual(report['status'], 'passed', report)
        self.assertTrue(older.is_dir())
        self.assertTrue(newest.is_dir())
        self.assertTrue(any(row['path'] == str(older) and row['status'] == 'retained'
                            for row in report['actions']))
        reclaimed = reclaim_abandoned(self.root, retained)
        self.assertEqual(reclaimed['status'], 'passed', reclaimed)
        self.assertEqual(reclaimed['bytes_reclaimed'], 0)
        self.assertEqual((output / 'category-cleanup.json').read_bytes(), receipt)
        self.assertTrue(newest.is_dir())

    def test_active_or_unknown_usage_preserves_build_and_fails(self):
        build = self.payload()
        for status in ('active', 'unavailable'):
            with self.subTest(status=status):
                report = cleanup_category(self.root, self.output, checker=lambda _: Usage(status, 'probe'))
                self.assertEqual(report['status'], 'failed')
                self.assertGreater(report['bytes_retained'], 0)
                self.assertEqual(report['unmeasured_paths'], [])
                self.assertTrue((build / 'payload').is_file())
                self.assertTrue(temporary_path(self.root, self.output).is_dir())
        process = self.hold_payload(build / 'payload')
        try:
            report = cleanup_category(self.root, self.output)
            self.assertEqual(report['status'], 'failed')
            row = next(row for row in report['actions'] if row['path'] == str(build))
            # An incomplete native inventory is also unsafe while the real FD is
            # held; both outcomes must preserve it, then become idle after exit.
            self.assertIn(row['status'], ('blocked', 'unavailable'), row)
            self.assertTrue((build / 'payload').is_file())
        finally:
            self.release_payload(process)
        report = cleanup_category(self.root, self.output)
        self.assertEqual(report['status'], 'passed', report)
        self.assertFalse(build.exists())
        self.assert_native_fixture_active_protection()

    def assert_native_fixture_active_protection(self):
        output = self.root / 'target/verification/native-active/acceptance'
        initialize_category(self.root, output, 'acceptance')
        self.track_temporary(output, 'acceptance')
        temporary = temporary_path(self.root, output)
        with NativeFixture(output / 'gate-held', 'native-held-', temporary) as fixture:
            payload = fixture / 'payload'
            payload.write_text('held native diagnostics')
            marker = output / 'gate-held/retained-fixture.json'
            marker.write_text(json.dumps(dict(path=str(self.root), gate_output=str(marker.parent),
                                               category_output=str(output))))
            unsafe = cleanup_category(self.root, output)
            self.assertEqual(unsafe['status'], 'failed')
            self.assertGreater(unsafe['bytes_retained'], 0)
            self.assertTrue(payload.is_file())
            marker.unlink()
            unavailable = cleanup_category(self.root, output,
                                           checker=lambda _: Usage('unavailable', 'OS inspection fault probe'))
            self.assertEqual(unavailable['status'], 'failed')
            self.assertGreater(unavailable['bytes_retained'], 0)
            self.assertTrue(payload.is_file())
            process = self.hold_payload(payload)
            try:
                active = cleanup_category(self.root, output)
                self.assertEqual(active['status'], 'failed', active)
                self.assertGreater(active['bytes_retained'], 0)
                self.assertTrue(payload.is_file())
                self.assertTrue(temporary.is_dir())
            finally:
                self.release_payload(process)
        self.assertEqual(cleanup_category(self.root, output)['status'], 'passed')
        self.assertFalse(temporary.exists())

    def hold_payload(self, path):
        code = ('import os,sys\n'
                'with open(sys.argv[1]) as held:\n'
                ' print(os.getpid(),flush=True)\n'
                ' sys.stdin.read()\n')
        process = subprocess.Popen([sys.executable, '-c', code, str(path)],
                                   stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.addCleanup(self.release_payload, process)
        self.assertTrue(select.select([process.stdout], [], [], 10)[0],
                        'Child did not acknowledge its open payload')
        self.assertEqual(int(process.stdout.readline()), process.pid)
        return process

    @staticmethod
    def release_payload(process):
        process.stdin.close()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
        finally:
            process.stdout.close()

    def test_explicit_retention_reports_paths_and_sizes_without_inspection(self):
        build = self.payload()
        report = cleanup_category(self.root, self.output, True,
                                  checker=lambda _: Usage('unavailable', 'OS inspection fault probe'))
        self.assertEqual(report['status'], 'passed')
        row = next(row for row in report['actions'] if row['path'] == str(build))
        self.assertEqual(row['status'], 'retained')
        self.assertGreater(row['estimated_bytes_after'], 0)
        external = temporary_path(self.root, self.output)
        self.assertTrue(external.is_dir())
        self.assertTrue(any(row['path'] == str(external) and row['status'] == 'retained'
                            for row in report['actions']))

    def test_symlinked_build_cannot_delete_external_payload(self):
        outside = self.root / 'external'
        outside.mkdir()
        (outside / 'payload').write_text('keep')
        (self.output / 'build').symlink_to(outside)
        report = cleanup_category(self.root, self.output)
        self.assertEqual(report['status'], 'failed')
        self.assertEqual((outside / 'payload').read_text(), 'keep')
        collision = self.root / 'target/verification/collision/unit'
        collision.mkdir(parents=True)
        (collision / 'results.json').write_text('[]')
        temporary = temporary_path(self.root, collision)
        temporary.symlink_to(outside)
        try:
            with self.assertRaises(FileExistsError):
                prepare_category(self.root, collision, 'unit')
            self.assertEqual((outside / 'payload').read_text(), 'keep')
        finally:
            temporary.unlink()

    def test_missing_or_forged_marker_never_authorizes_removal(self):
        build = self.payload()
        marker = self.output / 'category-workspace.json'
        original = marker.read_text()
        marker.unlink()
        self.assertEqual(cleanup_category(self.root, self.output)['status'], 'failed')
        marker.write_text(json.dumps({'version': 1, 'category': 'unit', 'output': str(self.root)}))
        self.assertEqual(cleanup_category(self.root, self.output)['status'], 'failed')
        self.assertTrue(build.exists())
        marker.write_text(original)
        temporary = temporary_path(self.root, self.output)
        external_marker = temporary / marker.name
        external_marker.write_text('{}')
        self.assertEqual(cleanup_category(self.root, self.output)['status'], 'failed')
        self.assertTrue(build.exists())
        self.assertTrue(temporary.exists())
        external_marker.write_text(original)

    def test_reclamation_visits_only_explicit_inactive_category_owners(self):
        old = self.output
        build = self.payload()
        current = self.root / 'target/verification/new/unit'
        current.mkdir(parents=True)
        (current / 'results.json').write_text('[]')
        prepare_category(self.root, current, 'unit')
        self.track_temporary(current)
        (current / 'build').mkdir()
        unowned = self.root / 'target/verification/unowned/build'
        unowned.mkdir(parents=True)
        nested = self.payload('tmp') / 'fixture/target/verification/nested/unit'
        nested.mkdir(parents=True)
        (nested / 'category-workspace.json').write_text('{}')
        report = reclaim_abandoned(self.root, current)
        self.assertEqual(report['status'], 'passed')
        self.assertFalse(build.exists())
        self.assertTrue((old / 'results.json').exists())
        self.assertTrue((current / 'build').exists())
        self.assertTrue(unowned.exists())
        receipt = old / 'category-cleanup.json'
        original = receipt.read_bytes()
        self.assertGreater(json.loads(original)['bytes_reclaimed'], 0)
        repeated = reclaim_abandoned(self.root, current)
        self.assertEqual(repeated['status'], 'passed')
        self.assertEqual(repeated['bytes_reclaimed'], 0)
        self.assertEqual(receipt.read_bytes(), original)
        (old / 'build').symlink_to(unowned)
        suspicious = reclaim_abandoned(self.root, current)
        self.assertEqual(suspicious['status'], 'failed')
        self.assertTrue(unowned.exists())
        self.assertEqual(receipt.read_bytes(), original)

    def test_unprepared_cleanup_accepts_only_absent_safe_directory(self):
        absent = self.root / 'target/verification/never-started/unit'
        report = cleanup_if_prepared(self.root, absent)
        self.assertEqual(report['status'], 'passed')
        self.assertFalse(absent.exists())
        absent.mkdir(parents=True)
        report = cleanup_if_prepared(self.root, absent)
        self.assertEqual(report['status'], 'failed')

    def test_reclamation_preserves_previous_explicit_diagnostic_retention(self):
        build = self.payload()
        retained = cleanup_category(self.root, self.output, True)
        current = self.root / 'target/verification/new/unit'
        current.mkdir(parents=True)
        (current / 'results.json').write_text('[]')
        prepare_category(self.root, current, 'unit')
        self.track_temporary(current)
        report = reclaim_abandoned(self.root, current,
                                   checker=lambda _: Usage('unavailable', 'OS inspection fault probe'))
        self.assertEqual(report['status'], 'passed')
        self.assertEqual(report['bytes_retained'], retained['bytes_retained'])
        self.assertGreater(report['bytes_retained'], 0)
        self.assertEqual(report['bytes_reclaimed'], 0)
        self.assertTrue((build / 'payload').is_file())
        self.assertTrue(temporary_path(self.root, self.output).exists())

    def test_real_timed_out_process_is_reaped_before_always_cleanup(self):
        # A real sleeping child probes timeout cleanup; it does not simulate business behavior.
        output = self.root / 'target/verification/runner/integration'
        self.track_temporary(output, 'integration')
        code = '\n'.join([
            'import json,os,pathlib,sys,time',
            'sys.path.insert(0, sys.argv[1])',
            'from verification.category_cleanup import prepare_category',
            'root,output=map(pathlib.Path,sys.argv[2:])',
            'prepare_category(root,output,"integration")',
            '(output/"build").mkdir()',
            '(output/"child.pid").write_text(str(os.getpid()))',
            '(output/"build/payload").write_text("compiled")',
            'time.sleep(30)',
        ])
        steps = [Step('held-child', [sys.executable, '-c', code, str(ROOT / 'tools'),
                                     str(self.root), str(output)], timeout=0.3),
                 Step('cleanup', [sys.executable,
                                  str(ROOT / 'tools/verification/category_cleanup.py'),
                                  '--root', str(self.root), '--output', str(output)],
                      always=True)]
        runner = Runner(self.root, output, steps)
        self.assertEqual(runner.run(), 1)
        # Some hosts deny killpg even outside the sandbox; that limitation must
        # stay explicit and unsuccessful while the directly owned child is reaped.
        self.assertIn(runner.results[0]['status'], ('timed-out', 'cleanup-unavailable'))
        self.assertEqual(runner.results[1]['status'], 'passed')
        pid = int((output / 'child.pid').read_text())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)
        self.assertFalse((output / 'build').exists())
        self.assertTrue((output / 'results.json').is_file())

    def test_sigterm_between_stage_records_preserves_evidence_and_runs_cleanup(self):
        # Real OS signals probe both active-process and durable-recording boundaries.
        for mode in ('saved', 'active'):
            with self.subTest(mode=mode):
                self.assert_signal_cleanup(mode)

    def assert_signal_cleanup(self, mode):
        output = self.root / f'target/verification/signal-{mode}/integration'
        self.track_temporary(output, 'integration')
        command = [sys.executable, '-c', self.signal_runner_code(), str(ROOT / 'tools'),
                   str(self.root), str(output), self.signal_child_code(), mode]
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if mode == 'active':
            self.signal_active_runner(process, output)
        _, errors = process.communicate(timeout=120)
        self.assertEqual(process.returncode, 1, errors)
        results = json.loads((output / 'results.json').read_text())
        expected = ('interrupted', 'cleanup-unavailable') if mode == 'active' else ('passed',)
        self.assertIn(results[0]['status'], expected)
        self.assertEqual(results[1]['status'], 'not-run')
        self.assertEqual(results[2]['status'], 'passed')
        self.assertFalse((output / 'build').exists())
        pid = int((output / 'child.pid').read_text())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)
        self.assertTrue((output / '01-prepare.log').is_file())

    def signal_active_runner(self, process, output):
        deadline = time.monotonic() + 10
        while not (output / 'child.pid').is_file():
            self.assertIsNone(process.poll(), 'Runner ended before the held child started')
            self.assertLess(time.monotonic(), deadline, 'Held child did not start')
            time.sleep(.01)
        process.terminate()

    @staticmethod
    def signal_child_code():
        return '\n'.join([
            'import os,pathlib,sys,time',
            'sys.path.insert(0,sys.argv[1])',
            'from verification.category_cleanup import prepare_category',
            'root,output=map(pathlib.Path,sys.argv[2:4])',
            'prepare_category(root,output,"integration")',
            '(output/"build").mkdir()',
            '(output/"build/payload").write_text("compiled")',
            '(output/"child.pid.pending").write_text(str(os.getpid()))',
            '(output/"child.pid.pending").replace(output/"child.pid")',
            'if sys.argv[4]=="active":time.sleep(30)',
        ])

    @staticmethod
    def signal_runner_code():
        # This hook controls signal timing, not business/dependency responses.
        return '\n'.join([
            'import os,pathlib,signal,sys',
            'sys.path.insert(0,sys.argv[1])',
            'from verification.runner import Runner,Step',
            'from verify import interrupt',
            'signal.signal(signal.SIGTERM,interrupt)',
            'class InterruptAtSave(Runner):',
            ' sent=False',
            ' def save(self):',
            '  super().save()',
            '  if sys.argv[5]=="saved" and self.results[0]["status"]=="passed" and not self.sent:',
            '   self.sent=True',
            '   os.kill(os.getpid(),signal.SIGTERM)',
            'root,output=map(pathlib.Path,sys.argv[2:4])',
            'steps=[Step("prepare",[sys.executable,"-c",sys.argv[4],sys.argv[1],str(root),str(output),sys.argv[5]]),',
            ' Step("later",[sys.executable,"-c","raise SystemExit(97)"]),',
            ' Step("cleanup",[sys.executable,str(pathlib.Path(sys.argv[1])/"verification/category_cleanup.py"),"--root",str(root),"--output",str(output)],always=True)]',
            'raise SystemExit(InterruptAtSave(root,output,steps).run())',
        ])


if __name__ == '__main__':
    unittest.main()
