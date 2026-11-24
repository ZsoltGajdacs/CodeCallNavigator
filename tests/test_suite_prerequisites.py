"""Selection policy is pure; resource availability uses real temporary paths."""
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.suite_prerequisites import configured_problems, node_supported, requirements_for, tools_for
from verification.typescript_compatibility import supports_version, supported_versions


class CompilerCompatibilityTests(unittest.TestCase):
    def test_stable_versions_follow_supported_minor_families(self):
        minors = ['5.9', '6.0']
        for version in ['5.9.0', '5.9.3', '5.9.99', '6.0.0', '6.0.3']:
            self.assertTrue(supports_version(version, minors), version)
        for version in ['5.8.3', '6.1.0', '7.0.0', '5.9', 'v5.9.3', '5.9.3-beta',
                        '6.0.0-dev.1', '5.9.3+build', '5.9.3\n', '05.9.3',
                        '5.09.3', '5.9.03', '5.9.3.1', '', None, 593]:
            self.assertFalse(supports_version(version, minors), version)

    def test_labels_and_eligibility_follow_supplied_policy(self):
        self.assertEqual(supported_versions(['5.9', '6.0']), '5.9.x or 6.0.x')
        self.assertEqual(supported_versions(['6.0']), '6.0.x')
        self.assertFalse(supports_version('5.9.3', ['6.0']))


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
            compiler = root / 'typescript/index.js'
            compiler.write_text("exports.version = '6.0.2';")
            self.assertEqual(configured_problems(root, {'typescript'}, None, environment), [])
            compiler.write_text("exports.version = '6.0.3';")
            self.assertIn('reports 6.0.3; package.json declares 6.0.2',
                          configured_problems(root, {'typescript'}, None, environment)[0])
            compiler.write_text("exports.version = '5.9.3';")
            (root / 'typescript/package.json').write_text('{"version":"5.9.3"}')
            self.assertEqual(configured_problems(root, {'typescript'}, None, environment), [])
            (root / 'typescript/package.json').write_text('{"version":"5.8.3"}')
            errors = configured_problems(root, {'typescript'}, None, environment)
            self.assertEqual(len(errors), 1)
            self.assertIn('5.9.x or 6.0.x', errors[0])


if __name__ == '__main__':
    unittest.main()
