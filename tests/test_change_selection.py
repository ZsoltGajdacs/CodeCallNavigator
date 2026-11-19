"""Real Git and shared-plan boundaries; pure tests cannot prove Git encoding or CLI side effects."""
from dataclasses import replace
import json
import os
import shlex
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
from verification.change_inputs import Change, discover_changes
from verification.change_policy import read_json, select_changes
from verification.change_plan import ChangePlan, build_steps, report_plan


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
        context = ChangePlan(args, 'HEAD', [Change(p, 'M', 'worktree') for p in paths],
                             self.policy, self.selection(*paths), self.inventory)
        output = ROOT / 'target/verification/PLAN'
        actual = build_steps(verify, context, output)
        expected = verify.plan(replace(args, mode='full'), paths[:1], output)
        self.assertEqual(actual, expected)
        self.assertFalse(any(s.command[0] == 'rustfmt' and 'vendor/' in s.command[-1] for s in actual))

    def test_focused_correctness_keeps_compiler_backed_tests_and_component_union(self):
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
        path = 'fixtures/typescript-corpus/src/app/page.tsx'
        context = ChangePlan(verify_changes.Options(), 'HEAD', [Change(path, 'M', 'worktree')],
                             self.policy, self.selection(path), self.inventory)
        steps = build_steps(verify, context, ROOT / 'target/verification/PLAN')
        self.assertIn('codescene', [step.name for step in steps])

    def test_development_plan_projects_final_selection_to_units(self):
        output = ROOT / 'target/verification/PLAN'
        for paths in (('tools/verify_changes.py',), ('src/view/activity.rs', 'tools/verify_changes.py'),
                      ('tests/test_change_selection.py',), ('README.md',), ('Cargo.lock',),
                      ('new/unknown.bin',)):
            with self.subTest(paths=paths):
                context = ChangePlan(verify_changes.Options(phase='development'), 'HEAD',
                                     [Change(path, 'M', 'worktree') for path in paths],
                                     self.policy, self.selection(*paths), self.inventory)
                steps = build_steps(verify, context, output)
                report = report_plan(context, steps)
                final = report_plan(replace(context, settings=verify_changes.Options()), [])
                self.assert_unit_projection(report, final)

    def assert_unit_projection(self, report, final):
        unit_ids = {row['id'] for row in self.inventory['tests'] if row['category'] == 'unit'}
        final_tests = set(final['selected']['tests'])
        self.assertEqual(report['phase'], 'development')
        self.assertEqual(report['scope'], final['scope'])
        self.assertEqual(report['excluded_by_policy'], final['excluded_by_policy'])
        self.assertEqual(set(report['selected']['tests']), final_tests & unit_ids)
        self.assertEqual(report['selected']['gates'], [])
        self.assertEqual(set(report['deferred_until_final']['tests']), final_tests - unit_ids)
        self.assertEqual(report['deferred_until_final']['gates'], final['selected']['gates'])
        self.assertTrue(all(selection['category'] == 'unit' for selection in report['selections']))
        self.assert_unit_stages(report['stages'], bool(final_tests & unit_ids))

    def assert_unit_stages(self, stages, has_units):
        names = {stage['name'] for stage in stages}
        if has_units:
            self.assertTrue({'unit-prerequisites', 'unit-prepare', 'unit-category-cleanup'} <= names)
            self.assertTrue(all(name.startswith('unit-') for name in names))
            self.assertFalse(any('bootstrap' in name or 'gate-' in name for name in names))
        else:
            self.assertEqual(names, set())

    def assert_docs_development_phase(self, command, env, root, final):
        output = root / 'target' / 'development'
        args = [*command, '--phase', 'development', '--output', str(output), '--format', 'json']
        report = subprocess.run(args, cwd='/', env=env, capture_output=True, text=True, check=True)
        plan = json.loads(report.stdout)
        self.assert_unit_projection(plan, final)
        self.assertTrue(plan['deferred_until_final']['checks'])
        self.assertIn('--phase', shlex.split(plan['execution_command']))
        final_command = shlex.split(plan['final_execution_command'])
        self.assertNotIn('--phase', final_command)
        self.assertNotIn('--output', final_command)
        self.assertFalse((root / 'target').exists())

        run = subprocess.run([*args, '--run'], cwd='/', env=env, capture_output=True, text=True, check=True)
        result = json.loads(run.stdout)
        self.assertEqual(result['result'], 'no-unit-tests')
        self.assertNotIn('evidence', result)
        self.assertFalse((root / 'target').exists())

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
            final_plan = json.loads(planned.stdout)
            self.assertEqual(final_plan['scope'], 'static')
            explicit = subprocess.run([*command, '--phase', 'final', '--format', 'json'],
                                      cwd='/', env=env, capture_output=True, text=True, check=True)
            self.assertEqual(json.loads(explicit.stdout), final_plan)
            self.assertFalse((root / 'target').exists())
            self.assert_docs_development_phase(command, env, root, final_plan)
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
