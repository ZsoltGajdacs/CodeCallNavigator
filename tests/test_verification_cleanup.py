"""Verification cleanup removes only idle owned artifacts and retains evidence."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.cleanup import Settings, Usage, cleanup_report, lsof_usage, check_usage

CARGO = r'''
import json, shutil, sys
from pathlib import Path
root = Path.cwd()
with (root / 'cargo-commands.jsonl').open('a') as stream:
    stream.write(json.dumps(sys.argv[1:]) + '\n')
arguments = sys.argv[1:]
target = Path(arguments[arguments.index('--target-dir') + 1])
profile = 'release' if '--release' in arguments else 'debug'
shutil.rmtree(target / profile)
if profile == 'debug' and (root / 'cargo-fails').exists():
    sys.exit(7)
'''


def idle(_):
    return Usage('idle')


class CleanupTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name).resolve()
        self.root = self.base / 'repo'
        self.output = self.root / 'target/verification/run'
        self.output.mkdir(parents=True)
        self.results([{'name': 'cargo-check', 'status': 'passed'}])
        self.cargo = self.root / 'fake-cargo'
        self.cargo.write_text(f'#!{sys.executable}\n' + CARGO)
        self.cargo.chmod(0o755)

    def results(self, rows):
        (self.output / 'results.json').write_text(json.dumps(rows))

    def data(self, relative):
        folder = self.root / relative
        folder.mkdir(parents=True, exist_ok=True)
        (folder / 'artifact').write_bytes(b'cache' * 20)
        return folder

    def run_cleanup(self, mode='light', checker=idle, keep=False):
        return cleanup_report(Settings(self.root, self.output, mode, keep, str(self.cargo)), checker)

    def test_light_removes_incremental_and_run_caches_but_keeps_dependencies_and_evidence(self):
        removed = [self.data(path) for path in (
            'target/debug/incremental', 'target/release/incremental',
            'target/verification/run/profile', 'target/verification/run/scale-cache',
        )]
        retained = [self.data(path) for path in (
            'target/debug/deps', 'target/release/deps', 'vendor/dependency', 'dist/CodeCallNavigator.app',
        )]
        (self.output / 'verification.log').write_text('evidence')
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'passed', report)
        self.assertTrue(all(not path.exists() for path in removed))
        self.assertTrue(all(path.exists() for path in retained))
        self.assertTrue((self.output / 'results.json').is_file())
        self.assertEqual((self.output / 'verification.log').read_text(), 'evidence')
        self.assertEqual(json.loads((self.output / 'cleanup.json').read_text()), report)
        self.assertTrue(all(row['estimated_bytes_before'] > 0 for row in report['actions']))
        self.assertTrue(all(row['estimated_bytes_after'] == 0 for row in report['actions']))

    def test_full_cargo_commands_are_profile_scoped_and_preserve_run_results(self):
        self.data('target/debug/deps')
        self.data('target/release/deps')
        retained = self.data('vendor/dependency')
        report = self.run_cleanup('full')
        self.assertEqual(report['status'], 'passed', report)
        commands = [json.loads(line) for line in (self.root / 'cargo-commands.jsonl').read_text().splitlines()]
        common = ['clean', '--locked', '--target-dir', str(self.root / 'target')]
        self.assertEqual(commands, [common + ['--profile', 'dev'], common + ['--release']])
        self.assertTrue((self.output / 'results.json').is_file())
        self.assertTrue(retained.exists())
        self.assertFalse((self.root / 'target/debug').exists())
        self.assertFalse((self.root / 'target/release').exists())

    def test_failed_cargo_cleanup_records_remaining_bytes_and_continues_other_profile(self):
        self.data('target/debug/deps')
        self.data('target/release/deps')
        (self.root / 'cargo-fails').touch()
        report = self.run_cleanup('full')
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(report['actions'][0]['status'], 'failed')
        self.assertEqual(report['actions'][0]['estimated_bytes_after'], 0)
        self.assertEqual(report['actions'][1]['status'], 'cleaned')
        self.assertTrue((self.output / 'cleanup-cargo-dev.log').is_file())

    def test_full_cleanup_cannot_remove_the_run_evidence_directory(self):
        self.output = self.root / 'target/debug/run'
        self.output.mkdir(parents=True)
        self.results([{'name': 'build-debug-engine', 'status': 'passed'}])
        report = self.run_cleanup('full')
        self.assertEqual(report['status'], 'failed')
        self.assertTrue((self.output / 'results.json').is_file())
        self.assertIn('own run evidence', report['actions'][0]['reason'])
        self.assertFalse((self.root / 'cargo-commands.jsonl').exists())

    def test_full_missing_prerequisites_preserves_existing_build_artifacts(self):
        debug = self.data('target/debug/deps')
        release = self.data('target/release/deps')
        scratch = self.data('target/verification/run/profile')
        self.results([{'name': 'prerequisites', 'status': 'failed'},
                      {'name': 'build-debug-engine', 'status': 'not-run'}])
        report = self.run_cleanup('full')
        self.assertEqual(report['status'], 'passed', report)
        self.assertTrue(debug.exists() and release.exists())
        self.assertFalse(scratch.exists())
        self.assertFalse((self.root / 'cargo-commands.jsonl').exists())
        self.assertEqual([row['status'] for row in report['actions'][:2]], ['retained', 'retained'])

    def test_active_candidate_is_blocked_while_independent_idle_cleanup_continues(self):
        debug = self.data('target/debug/incremental')
        release = self.data('target/release/incremental')
        def checker(path):
            return Usage('active', 'Open by a process') if path == debug else Usage('idle')
        report = self.run_cleanup(checker=checker)
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(debug.exists())
        self.assertFalse(release.exists())
        self.assertEqual(report['actions'][0]['status'], 'blocked')
        self.assertTrue((self.output / 'cleanup.json').is_file())

    def test_unknown_process_inspection_never_allows_deletion(self):
        candidate = self.data('target/debug/incremental')
        report = self.run_cleanup(checker=lambda _: Usage('unavailable', 'Inspection failed'))
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(candidate.exists())
        self.assertEqual(report['actions'][0]['status'], 'unavailable')

    def test_keep_artifacts_does_not_require_process_inspection_or_delete_anything(self):
        candidate = self.data('target/debug/incremental')
        scratch = self.data('target/verification/run/profile')
        def forbidden(_):
            self.fail('Retained artifacts do not need a deletion process check')
        report = self.run_cleanup(checker=forbidden, keep=True)
        self.assertEqual(report['status'], 'passed', report)
        self.assertTrue(candidate.exists() and scratch.exists())
        self.assertEqual(report['actions'][0]['status'], 'retained')

    def test_external_owned_run_directory_is_supported_but_unowned_output_is_rejected(self):
        self.output = self.base / 'external-run'
        self.output.mkdir()
        self.results([])
        scratch = self.output / 'profile'
        scratch.mkdir()
        (scratch / 'graph').write_text('copied graph')
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'passed', report)
        self.assertFalse(scratch.exists())
        self.assertTrue((self.output / 'results.json').exists())
        (self.output / 'results.json').unlink()
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'failed')
        self.assertIn('ownership marker', report['errors'][0])

    def test_symlink_escape_is_rejected_and_external_payload_is_untouched(self):
        external = self.base / 'external'
        external.mkdir()
        payload = external / 'artifact'
        payload.write_text('shared cache')
        (self.root / 'target/debug').symlink_to(external, target_is_directory=True)
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'failed')
        self.assertEqual(payload.read_text(), 'shared cache')
        self.assertEqual(report['actions'][0]['status'], 'failed')

    def test_root_output_and_run_cache_symlinks_are_rejected(self):
        alias = self.base / 'alias'
        alias.symlink_to(self.root, target_is_directory=True)
        report = cleanup_report(Settings(alias, self.output, 'light'), idle)
        self.assertEqual(report['status'], 'failed')
        external = self.base / 'external'
        external.mkdir()
        (self.output / 'profile').symlink_to(external, target_is_directory=True)
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'failed')
        self.assertTrue(external.exists())
        output_alias = self.base / 'run-alias'
        output_alias.symlink_to(self.output, target_is_directory=True)
        report = cleanup_report(Settings(self.root, output_alias, 'light'), idle)
        self.assertEqual(report['status'], 'failed')

    def test_dist_symlink_failure_does_not_prevent_independent_run_cleanup(self):
        scratch = self.data('target/verification/run/profile')
        external = self.base / 'external-dist'
        external.mkdir()
        (self.root / 'dist').symlink_to(external, target_is_directory=True)
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'failed')
        self.assertFalse(scratch.exists())
        self.assertTrue(external.exists())
        self.assertTrue((self.output / 'cleanup.json').is_file())

    def test_nested_symlink_is_unlinked_without_following_external_cache(self):
        candidate = self.data('target/debug/incremental')
        external = self.base / 'external'
        external.mkdir()
        payload = external / 'artifact'
        payload.write_text('shared cache')
        (candidate / 'shared').symlink_to(external, target_is_directory=True)
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'passed', report)
        self.assertFalse(candidate.exists())
        self.assertEqual(payload.read_text(), 'shared cache')

    def test_package_retention_keeps_newest_per_prefix_and_current_outputs(self):
        newest, oldest = [], []
        for prefix in ('CodeCallNavigator', 'CodeGraph'):
            old = self.data(f'dist/{prefix}.previous-old.app')
            new = self.data(f'dist/{prefix}.previous-new.app')
            os.utime(old, ns=(1, 1))
            os.utime(new, ns=(2, 2))
            oldest.append(old)
            newest.append(new)
        for name in ('CodeCallNavigator.app', 'CodeCallNavigator.zip', 'CodeCallNavigator.dmg'):
            (self.root / 'dist' / name).write_text('current output')
        report = self.run_cleanup()
        self.assertEqual(report['status'], 'passed', report)
        self.assertTrue(all(path.exists() for path in newest))
        self.assertTrue(all(not path.exists() for path in oldest))
        self.assertTrue((self.root / 'dist/CodeCallNavigator.zip').is_file())
        self.assertTrue((self.root / 'dist/CodeCallNavigator.dmg').is_file())
        self.assertTrue((self.root / 'dist/CodeCallNavigator.app').is_file())

    def test_native_lsof_timeout_or_launch_error_is_unavailable(self):
        for error in (OSError('cannot launch'), subprocess.TimeoutExpired('lsof', 30)):
            with self.subTest(error=type(error).__name__):
                with patch('verification.cleanup.sys.platform', 'darwin'), \
                     patch('verification.cleanup.subprocess.run', side_effect=error):
                    self.assertEqual(check_usage(self.output).status, 'unavailable')

    def test_standalone_cli_keeps_artifacts_and_reports_invalid_ownership_nonzero(self):
        candidate = self.data('target/debug/incremental')
        script = Path(__file__).resolve().parents[1] / 'tools/verification/cleanup.py'
        command = [sys.executable, str(script), '--root', str(self.root),
                   '--output', str(self.output), '--mode', 'light', '--keep-artifacts']
        passed = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertEqual(passed.returncode, 0, passed.stderr)
        self.assertEqual(json.loads(passed.stdout)['status'], 'passed')
        self.assertTrue(candidate.exists())
        (self.output / 'results.json').unlink()
        failed = subprocess.run(command, capture_output=True, text=True, timeout=10)
        self.assertNotEqual(failed.returncode, 0)
        self.assertEqual(json.loads(failed.stdout)['status'], 'failed')
        self.assertTrue(candidate.exists())

    def test_lsof_failure_or_warning_cannot_be_mistaken_for_idle(self):
        for code, stdout, stderr in [(2, '', ''), (1, '', 'cannot inspect'),
                                     (0, '', ''), (1, 'partial result', '')]:
            with self.subTest(code=code, stdout=stdout, stderr=stderr):
                result = subprocess.CompletedProcess([], code, stdout, stderr)
                self.assertEqual(lsof_usage(result).status, 'unavailable')
        self.assertEqual(lsof_usage(subprocess.CompletedProcess([], 1, '', '')).status, 'idle')
        self.assertEqual(lsof_usage(subprocess.CompletedProcess([], 0, 'p123\n', '')).status, 'active')


if __name__ == '__main__':
    unittest.main()
