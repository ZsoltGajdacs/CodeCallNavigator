"""Selection policy is pure; resource availability uses real temporary paths."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.suite_prerequisites import configured_problems, node_supported, requirements_for, tools_for


class RequirementPolicyTests(unittest.TestCase):
    def test_requirement_union_expands_typescript_without_unrelated_toolchains(self):
        rows = [dict(requirements=['typescript']), dict(requirements=['git'])]
        self.assertEqual(requirements_for(rows, []), {'typescript', 'node', 'git'})
        self.assertEqual(requirements_for([], []), set())

    def test_rust_build_requires_compiler_and_cleanup_without_private_compilers(self):
        tools = tools_for(set(), {'rust'})
        self.assertEqual(tools, {'cargo', 'rustc', '/usr/sbin/lsof'})
        self.assertNotIn('java', tools)
        self.assertNotIn('dotnet', tools)
        self.assertNotIn('mvn', tools)

    def test_packaged_gate_needs_build_and_signing_tools_without_existing_cli(self):
        requirements = requirements_for([], [dict(requirements=['packaged'])])
        self.assertTrue({'java', 'dotnet', 'typescript', 'node', 'gradle'}.issubset(requirements))
        self.assertNotIn('private', requirements)
        tools = tools_for({'packaged'}, set())
        self.assertTrue({'cargo', 'rustc', 'codesign', 'otool', 'install_name_tool'}.issubset(tools))
        self.assertNotIn('codecallnavigator-engine', tools)
        self.assertNotIn('ccnavigator', tools)

    def test_node_version_rejects_old_and_malformed_versions(self):
        self.assertTrue(node_supported('v24.1.0\n'))
        self.assertTrue(node_supported('25.0.1'))
        self.assertFalse(node_supported('v23.9.0'))
        self.assertFalse(node_supported('unrecognized'))


class SelectedResourceTests(unittest.TestCase):
    def test_private_requirement_checks_real_checkout_without_unrelated_modules(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = {'CODECALLNAVIGATOR_RECURRING_BILLING': str(root)}
            self.assertEqual(configured_problems(root, set(), None, {}), [])
            errors = configured_problems(root, {'private'}, None, environment)
            self.assertEqual(len(errors), 1)
            self.assertIn('pom.xml', errors[0])
            (root / 'pom.xml').write_text('<project/>')
            self.assertEqual(configured_problems(root, {'private'}, None, environment), [])

    def test_typescript_checks_selected_package_versions_without_private_checkout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            environment = {'CODECALLNAVIGATOR_TS_NODE_MODULES': str(root)}
            for name, version in [('typescript', '6.0.2'), ('react', '19.1.0'), ('next', '16.1.0')]:
                (root / name).mkdir()
                (root / name / 'package.json').write_text('{"version":"' + version + '"}')
            self.assertEqual(configured_problems(root, {'typescript'}, None, environment), [])
            (root / 'typescript/package.json').write_text('{"version":"5.9.0"}')
            errors = configured_problems(root, {'typescript'}, None, environment)
            self.assertEqual(len(errors), 1)
            self.assertIn('expected 6.0.', errors[0])


if __name__ == '__main__':
    unittest.main()
