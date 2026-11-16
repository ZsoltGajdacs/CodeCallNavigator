"""Real Git and shared-plan boundaries; pure tests cannot prove Git encoding or CLI side effects."""
from dataclasses import replace
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import verify
import verify_changes
from verification.change_inputs import discover_changes
from verification.change_policy import read_json, select_changes
from verification.change_plan import ChangePlan, build_steps


class GitSelectionTests(unittest.TestCase):
    def test_index_worktree_rename_vendor_and_untracked_boundaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.DEVNULL)
            git('init', '-q')
            for name in ('restore.rs', 'delete.rs', 'old.rs', 'vendor/upstream.rs', '.gitignore'):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('ignored.rs\n' if name == '.gitignore' else 'original\n')
            git('add', '.')
            git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'base')
            base = git('rev-parse', 'HEAD').decode().strip()
            (root / 'restore.rs').write_text('staged\n')
            git('add', 'restore.rs')
            (root / 'restore.rs').write_text('original\n')
            git('mv', 'old.rs', 'new\nname.rs')
            (root / 'delete.rs').unlink()
            (root / 'vendor/upstream.rs').write_text('modified\n')
            (root / 'untracked space.rs').write_text('new\n')
            (root / 'ignored.rs').write_text('ignored\n')
            revision, changes = discover_changes(root, 'HEAD')
            self.assertEqual(revision, base)
            self.assertEqual({c.path for c in changes}, {'restore.rs', 'delete.rs', 'old.rs',
                             'new\nname.rs', 'vendor/upstream.rs', 'untracked space.rs'})
            self.assertEqual({c.origin for c in changes if c.path == 'restore.rs'}, {'index', 'worktree'})
            git('add', '.')
            git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'changes')
            self.assertEqual(discover_changes(root, 'HEAD')[1], [])
            self.assertTrue(discover_changes(root, base)[1])
            with self.assertRaises(subprocess.CalledProcessError):
                discover_changes(root, 'missing-reference')


class RepositoryPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = read_json(ROOT / 'tools/verification/change-rules.json')
        cls.inventory = read_json(ROOT / 'tools/verification/test-inventory.json')

    def selection(self, *paths):
        return select_changes(paths, self.policy, self.inventory, self.inventory)

    def test_actual_full_triggers_include_dependencies_vendor_and_unknown(self):
        for path in ('Cargo.lock', 'crates/codecallnavigator-core/Cargo.toml',
                     'tools/bootstrap.py', 'tools/package.py', '.github/workflows/release.yml',
                     'patches/new.patch', 'vendor/rune/src/lib.rs', 'new/unknown.bin'):
            with self.subTest(path=path):
                self.assertTrue(self.selection(path).full)

    def test_verification_tooling_selects_no_application_build_or_runtime_checks(self):
        from verification.change_inputs import Change
        for path in ('tools/verify_changes.py', 'tools/verification/change-rules.json',
                     'tools/verification/suites.py', 'verify-changes.sh'):
            with self.subTest(path=path):
                decision = self.selection(path)
                self.assertFalse(decision.full)
                self.assertEqual({c for _, c in decision.pairs}, {'tooling'})
                context = ChangePlan(verify_changes.Options(), 'HEAD', [Change(path, 'M', 'worktree')],
                                     self.policy, decision, self.inventory)
                steps = build_steps(verify, context, ROOT / 'target/verification/PLAN')
                self.assertNotIn('cargo', {step.command[0] for step in steps})
                names = {step.name for step in steps}
                forbidden = {'unit-compile-rust', 'integration-compile-rust', 'integration-compile-cli',
                             'acceptance-package', 'performance-compile-rust'}
                self.assertTrue(names.isdisjoint(forbidden))
                self.assertTrue({'unit-test-python', 'integration-test-python'} <= names)

    def test_actual_component_consumers_and_performance_triggers(self):
        ui = self.selection('src/view/activity.rs')
        self.assertEqual({c for _, c in ui.pairs}, {'ui'})
        self.assertFalse(any(s == 'performance' for s, _ in ui.pairs))
        core = self.selection('crates/codecallnavigator-core/src/model.rs')
        self.assertEqual({c for _, c in core.pairs}, {'core', 'engine', 'ui', 'mcp'})
        for path, component in (('src/view/graph_canvas/cards.rs', 'ui'),
                                ('crates/codecallnavigator-core/src/store.rs', 'core'),
                                ('crates/codecallnavigator-engine/src/entrypoints/query.rs', 'engine')):
            with self.subTest(path=path):
                self.assertIn(('performance', component), self.selection(path).pairs)

    def test_auto_full_stages_equal_legacy_and_skip_vendor_formatting(self):
        args = verify_changes.Options()
        paths = ['Cargo.lock', 'vendor/rune/src/lib.rs']
        from verification.change_inputs import Change
        context = ChangePlan(args, 'HEAD', [Change(p, 'M', 'worktree') for p in paths],
                             self.policy, self.selection(*paths), self.inventory)
        output = ROOT / 'target/verification/PLAN'
        actual = build_steps(verify, context, output)
        expected = verify.plan(replace(args, mode='full'), paths[:1], output)
        self.assertEqual(actual, expected)
        self.assertFalse(any(s.command[0] == 'rustfmt' and 'vendor/' in s.command[-1] for s in actual))

    def test_focused_correctness_keeps_compiler_backed_tests_and_component_union(self):
        from verification.change_inputs import Change
        path = 'crates/codecallnavigator-core/src/model.rs'
        context = ChangePlan(verify_changes.Options(), 'HEAD', [Change(path, 'M', 'worktree')],
                             self.policy, self.selection(path), self.inventory)
        steps = build_steps(verify, context, ROOT / 'target/verification/PLAN')
        self.assertFalse(any('--ordinary' in s.command for s in steps))
        self.assertEqual(sum(s.name == 'integration-compile-rust' for s in steps), 1)
        build = next(s for s in steps if s.name == 'integration-compile-rust')
        self.assertEqual(build.command.count('--component'), 4)
        self.assertEqual(len({s.name for s in steps}), len(steps))

    def test_tsx_fixtures_receive_shared_source_quality_review(self):
        from verification.change_inputs import Change
        path = 'fixtures/typescript-corpus/src/app/page.tsx'
        context = ChangePlan(verify_changes.Options(), 'HEAD', [Change(path, 'M', 'worktree')],
                             self.policy, self.selection(path), self.inventory)
        steps = build_steps(verify, context, ROOT / 'target/verification/PLAN')
        self.assertIn('codescene', [step.name for step in steps])

    def test_cli_text_json_and_execute_clean_checkout_without_artifacts(self):
        # A copied minimal real checkout proves CLI behavior without altering this checkout's Git state.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copytree(ROOT / 'tools/verification', root / 'tools/verification',
                            ignore=shutil.ignore_patterns('__pycache__'))
            for name in ('verify.py', 'verify_changes.py'):
                shutil.copyfile(ROOT / 'tools' / name, root / 'tools' / name)
            shutil.copyfile(ROOT / 'verify-changes.sh', root / 'verify-changes.sh')
            (root / '.gitignore').write_text('target/\n__pycache__/\n')
            subprocess.run(['git', 'init', '-q', str(root)], check=True)
            subprocess.run(['git', 'add', '.'], cwd=root, check=True)
            subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                            'commit', '-qm', 'base'], cwd=root, check=True, capture_output=True)
            env = os.environ | {'PYTHON': sys.executable, 'PYTHONDONTWRITEBYTECODE': '1'}
            command = ['bash', str(root / 'verify-changes.sh')]
            result = subprocess.run([*command, '--format', 'json'], cwd='/', env=env,
                                    capture_output=True, text=True, check=True)
            report = json.loads(result.stdout)
            self.assertEqual(report['scope'], 'empty')
            self.assertEqual(report['stages'], [])
            self.assertFalse((root / 'target').exists())
            (root / 'README.md').write_text('# Documentation\n')
            planned = subprocess.run([*command, '--format', 'json'], cwd='/', env=env,
                                     capture_output=True, text=True, check=True)
            self.assertEqual(json.loads(planned.stdout)['scope'], 'static')
            self.assertFalse((root / 'target').exists())
            executed = subprocess.run([*command, '--run', '--format', 'json'], cwd='/', env=env,
                                      capture_output=True, text=True, check=True)
            final = json.loads(executed.stdout)
            self.assertEqual(final['result'], 'passed')
            self.assertTrue((Path(final['evidence']) / 'change-plan.json').is_file())
            self.assertFalse(list(root.rglob('build')))
            text = subprocess.run(command, cwd='/', env=env, capture_output=True, text=True, check=True)
            self.assertIn('Verification: static', text.stdout)
            self.assertIn('documentation-static', text.stdout)
            rejected = subprocess.run([*command, '--full'], env=env, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)


if __name__ == '__main__':
    unittest.main()
