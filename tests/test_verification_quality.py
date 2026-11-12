"""Quality gates use real bounded stdio sessions and fail closed on incomplete evidence."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.quality import run_checks

SERVER = r'''
import json, os, subprocess, sys, time
from pathlib import Path
root = Path.cwd()
config = json.loads((root / 'server.json').read_text())
def publish_pid(name, pid):
    pending = root / (name + '.pending')
    pending.write_text(str(pid))
    pending.replace(root / name)
publish_pid('server.pid', os.getpid())
initialized = False
for line in sys.stdin:
    request = json.loads(line)
    method = request['method']
    if method == 'notifications/initialized':
        initialized = True
        continue
    if config.get('hang'):
        child = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])
        publish_pid('child.pid', child.pid)
        time.sleep(30)
    if config.get('exit'):
        sys.stderr.write('private server diagnostics must not leak')
        sys.exit(7)
    if method == 'initialize':
        result = {'protocolVersion': '2024-11-05', 'capabilities': {}, 'serverInfo': {'name': 'fixture', 'version': '1'}}
    elif not initialized:
        result = {'isError': True}
    elif request['params']['name'] == 'code_health_review':
        document = {'score': 10.0, 'review': [], 'path': request['params']['arguments']['file_path']}
        document.update(config.get('review', {}))
        document.update(config.get('reviews', {}).get(Path(document['path']).name, {}))
        result = {'content': [{'type': 'text', 'text': 'Version update available\n' + json.dumps(document)}]}
    else:
        document = {'results': [], 'quality_gates': 'passed', 'metadata': {
            'checked-file-count': 1, 'code-health-eligible-file-count': 1,
            'total-modified-file-count': 1, 'status': 'no-issues-found'}}
        document.update(config.get('safeguard', {}))
        result = {'structuredContent': document}
    if config.get('is_error') and method != 'initialize':
        result['isError'] = True
    if config.get('malformed'):
        print('not protocol JSON', flush=True)
    else:
        print(json.dumps({'jsonrpc': '2.0', 'method': 'notifications/message', 'params': {}}), flush=True)
        print(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}), flush=True)
'''


class QualitySessionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.command = self.root / 'fake-codescene'
        self.command.write_text(f'#!{sys.executable}\n' + SERVER)
        self.command.chmod(0o755)
        (self.root / 'changed.py').write_text('print("changed")\n')
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)
        self.commit_source()
        (self.root / 'changed.py').write_text('print("local change")\n')

    def commit_source(self):
        subprocess.run(['git', 'add', 'changed.py'], cwd=self.root, check=True)
        subprocess.run([
            'git', '-c', 'user.name=Verification Fixture', '-c',
            'user.email=verification@example.invalid', '-c', 'core.hooksPath=/dev/null',
            '-c', 'commit.gpgsign=false', 'commit', '-qm', 'Fixture source',
        ], cwd=self.root, check=True)

    def check(self, config=None, files=None, timeout=3):
        (self.root / 'server.json').write_text(json.dumps(config or {}))
        return run_checks(self.root, files or ['changed.py'], str(self.command), timeout)

    def test_complete_reviews_and_safeguard_pass_with_banner_and_notifications(self):
        report = self.check()
        self.assertEqual(report['status'], 'passed', report)
        self.assertEqual(report['reviews'][0]['score'], 10)
        self.assertEqual(report['safeguard']['metadata']['checked-file-count'], 1)
        self.assertEqual(report['errors'], [])

    def test_committed_source_is_reviewed_without_requiring_local_safeguard_coverage(self):
        self.commit_source()
        report = self.check({'safeguard': {'metadata': {
            'checked-file-count': 0, 'code-health-eligible-file-count': 0,
            'total-modified-file-count': 0, 'status': 'no-applicable-files',
        }}})
        self.assertEqual(report['status'], 'passed', report)
        self.assertEqual([review['path'] for review in report['reviews']], ['changed.py'])
        self.assertEqual(report['safeguard']['metadata']['checked-file-count'], 0)

    def test_review_failure_names_its_file_and_preserves_prior_successes(self):
        (self.root / 'new.py').write_text('print("new")\n')
        report = self.check({'reviews': {'new.py': {'score': '-'}}},
                            files=['changed.py', 'new.py'])
        self.assertEqual(report['status'], 'failed')
        self.assertIn('new.py', report['errors'][0])
        self.assertIn('numeric score', report['errors'][0])
        self.assertEqual([review['path'] for review in report['reviews']], ['changed.py'])
        self.assertIsNone(report['safeguard'])

    def test_findings_tool_errors_and_regressions_fail(self):
        for config in [
            {'review': {'review': [{'category': 'Complex Method'}]}},
            {'is_error': True},
            {'safeguard': {'quality_gates': 'failed'}},
            {'safeguard': {'results': [{'verdict': 'degraded'}]}},
            {'safeguard': {'results': [{'verdict': 'unknown'}]}},
        ]:
            with self.subTest(config=config):
                self.assertEqual(self.check(config)['status'], 'failed')

    def test_invalid_result_shapes_fail_as_structured_errors(self):
        for config in [
            {'review': {'score': None}},
            {'review': {'score': 'unknown'}},
            {'review': {'score': True}},
            {'review': {'score': -1}},
            {'review': {'review': None}},
            {'safeguard': {'metadata': None}},
            {'safeguard': {'results': None}},
            {'safeguard': {'results': [None]}},
            {'safeguard': {'results': [{}]}},
        ]:
            with self.subTest(config=config):
                report = self.check(config)
                self.assertEqual(report['status'], 'failed')
                self.assertTrue(report['errors'])

    def test_failed_safeguard_evidence_is_retained_for_the_report(self):
        report = self.check({'safeguard': {'quality_gates': 'failed'}})
        self.assertEqual(report['safeguard']['quality_gates'], 'failed')
        self.assertEqual(report['status'], 'failed')

    def test_empty_or_partial_coverage_cannot_pass_changed_tracked_sources(self):
        for metadata in [
            {},
            {'checked-file-count': 0, 'code-health-eligible-file-count': 0,
             'total-modified-file-count': 1, 'status': 'no-applicable-files'},
            {'checked-file-count': 1, 'code-health-eligible-file-count': 2,
             'total-modified-file-count': 2, 'status': 'incomplete'},
        ]:
            with self.subTest(metadata=metadata):
                report = self.check({'safeguard': {'metadata': metadata}})
                self.assertEqual(report['status'], 'failed')
                self.assertTrue(report['errors'])

    def test_staged_local_source_still_requires_safeguard_coverage(self):
        subprocess.run(['git', 'add', 'changed.py'], cwd=self.root, check=True)
        report = self.check({'safeguard': {'metadata': {
            'checked-file-count': 0, 'code-health-eligible-file-count': 0,
            'total-modified-file-count': 0, 'status': 'no-applicable-files',
        }}})
        self.assertEqual(report['status'], 'failed')
        self.assertIn('modified tracked source', report['errors'][0])

    def test_untracked_files_are_reviewed_without_inventing_safeguard_coverage(self):
        (self.root / 'new.py').write_text('print("new")\n')
        report = self.check({'safeguard': {'metadata': {
            'checked-file-count': 0, 'code-health-eligible-file-count': 0,
            'total-modified-file-count': 0, 'status': 'no-applicable-files',
        }}}, files=['new.py'])
        self.assertEqual(report['status'], 'passed', report)
        self.assertEqual(len(report['reviews']), 1)
        self.assertEqual(report['safeguard']['metadata']['checked-file-count'], 0)

    def test_deleted_and_unsupported_paths_are_explicitly_skipped(self):
        report = self.check(files=['changed.py', 'gone.py', 'README.md'])
        self.assertEqual(report['status'], 'passed', report)
        self.assertEqual(report['skipped_files'], ['README.md', 'gone.py'])

    def test_null_rust_declaration_review_records_applicability_and_truthful_coverage(self):
        source = self.root / 'flow.rs'
        source.write_text('//! Flow contract.\n/* Nested /* Rust */ comment */\nmod builder;\npub(crate) mod model;\npub use builder::{build, Item};\npub const MAX_STEPS: usize = 1_500;\n')
        subprocess.run(['git', 'add', 'flow.rs'], cwd=self.root, check=True)
        report = self.check({'review': {'score': None}, 'safeguard': {'metadata': {
            'checked-file-count': 0, 'code-health-eligible-file-count': 0,
            'total-modified-file-count': 1, 'status': 'no-applicable-files',
        }}}, files=['flow.rs'])
        self.assertEqual(report['status'], 'passed', report)
        self.assertIsNone(report['reviews'][0]['score'])
        self.assertEqual(report['reviews'][0]['applicability'], 'declarations_only')
        self.assertEqual(report['safeguard']['metadata']['checked-file-count'], 0)

    def test_null_score_cannot_skip_executable_or_unrecognized_rust(self):
        for content in [
            'pub fn execute() {}\n',
            '#[cfg(test)] mod tests;\n',
            'include!("generated.rs");\n',
            'pub const SIZE: usize = compute();\n',
            'mod implementation { fn execute() {} }\n',
            'unknown declarations;\n',
            '/* unfinished comment\nmod builder;\n',
        ]:
            with self.subTest(content=content):
                (self.root / 'unknown.rs').write_text(content)
                report = self.check({'review': {'score': None}}, files=['unknown.rs'])
                self.assertEqual(report['status'], 'failed', report)
                self.assertIn('numeric score', report['errors'][0])

    def test_null_declaration_review_still_requires_well_formed_empty_findings(self):
        (self.root / 'flow.rs').write_text('pub mod builder;\n')
        for findings in [None, {}, [{'category': 'Complex Method'}]]:
            with self.subTest(findings=findings):
                report = self.check({'review': {'score': None, 'review': findings}}, files=['flow.rs'])
                self.assertEqual(report['status'], 'failed', report)

    def test_protocol_corruption_and_premature_exit_fail_without_diagnostics_leak(self):
        for config in [{'malformed': True}, {'exit': True}]:
            with self.subTest(config=config):
                report = self.check(config)
                self.assertEqual(report['status'], 'failed')
                self.assertNotIn('private server diagnostics', json.dumps(report))
                self.assert_process_stopped('server.pid')

    def test_denied_process_group_cleanup_cannot_report_success(self):
        with patch('verification.mcp_stdio.os.killpg', side_effect=PermissionError):
            report = self.check()
        self.assertEqual(report['status'], 'failed')
        self.assertIn('cleanup unavailable', report['errors'][0])
        self.assert_process_stopped('server.pid')

    def test_timeout_stops_server_and_children(self):
        started = time.monotonic()
        # Include interpreter startup under full-build load; the three-second
        # assertion still proves that the 30-second fixture and child are stopped.
        report = self.check({'hang': True}, timeout=1.0)
        self.assertEqual(report['status'], 'failed')
        self.assertIn('timed out', report['errors'][0])
        self.assertLess(time.monotonic() - started, 3)
        self.assert_process_stopped('server.pid')
        self.assert_process_stopped('child.pid')

    def assert_process_stopped(self, file):
        pid = int((self.root / file).read_text())
        for _ in range(20):
            try:
                os.kill(pid, 0)
            except ProcessLookupError:
                return
            time.sleep(0.01)
        self.fail(f'{file} process remained after session cleanup')


if __name__ == '__main__':
    unittest.main()
