"""Exact source ownership and inventory drift prevent silent coverage omissions."""
from pathlib import Path
import json
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.inventory import (
    Addition, add_tests, selected, unique_rows, validate_inventory,
    validate_metadata, verify_registered,
)
from verification.inventory_discovery import discover_tests, rust_mask


class RustLexingTests(unittest.TestCase):
    def test_comments_raw_strings_and_characters_cannot_register_tests(self):
        source = '''/* outer /* #[test] */ fn fake() {} */
const VALUE: &str = r###"#[test] fn nope() { }"###;
const CHARACTER: char = '}';
#[test] fn real() {}'''
        masked = rust_mask(source)
        self.assertEqual(len(masked), len(source))
        self.assertEqual(masked.count('#[test]'), 1)
        self.assertIn('fn real()', masked)
        self.assertEqual(masked.count('\n'), source.count('\n'))

    def test_lifetimes_do_not_hide_later_code(self):
        source = "fn borrow<'a>(value: &'a str) {}\n#[test] fn actual() {}"
        self.assertIn("<'a>", rust_mask(source))
        self.assertIn('#[test] fn actual()', rust_mask(source))


class InventoryPolicyTests(unittest.TestCase):
    def row(self, **values):
        return dict(id='example', component='core', category='unit', requirements=[],
                    purpose='Reject invalid input', **values)

    def test_duplicate_identities_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Duplicate inventory'):
            unique_rows([self.row(), self.row()])

    def test_unit_tests_cannot_request_external_boundaries(self):
        row = self.row()
        row['requirements'] = ['sqlite']
        with self.assertRaisesRegex(ValueError, 'external boundary'):
            validate_metadata(row)

    def test_unknown_category_and_missing_purpose_are_rejected(self):
        from verification.suite_execution import appended_environment
        base = {'MAVEN_OPTS': '-Xmx2g', 'OTHER': 'preserve'}
        scoped = appended_environment({'id': 'private-fixture', 'environment_append': {'MAVEN_OPTS': '-DskipUi'}}, base)
        self.assertEqual(scoped, {'MAVEN_OPTS': '-Xmx2g -DskipUi', 'OTHER': 'preserve'})
        self.assertEqual(base, {'MAVEN_OPTS': '-Xmx2g', 'OTHER': 'preserve'})
        row = self.row()
        row['category'] = 'fast'
        with self.assertRaisesRegex(ValueError, 'category'):
            validate_metadata(row)
        row['category'] = 'unit'
        row['purpose'] = ' '
        with self.assertRaisesRegex(ValueError, 'purpose'):
            validate_metadata(row)
        row['purpose'] = 'Reject invalid input'
        row['runner'] = 'rust'
        for environment in ([], {'bad-name': 'value'}, {1: 'value'}, {'MAVEN_OPTS': 1},
                            {'MAVEN_OPTS': '-DskipUi\x00'}):
            row['environment_append'] = environment
            with self.subTest(environment=environment), self.assertRaisesRegex(ValueError, 'environment'):
                validate_metadata(row)
        row['environment_append'] = {'MAVEN_OPTS': '-DskipUi'}
        validate_metadata(row)
        self.assert_serial_metadata(row)

    def assert_serial_metadata(self, row):
        row = row | {'category': 'integration'}
        for value in (None, '', ' ', 1, [], {}):
            with self.subTest(serial_group=value), self.assertRaisesRegex(ValueError, 'serial group'):
                validate_metadata(row | {'serial_group': value})
        validate_metadata(row | {'serial_group': 'shared-fixture'})
        with self.assertRaisesRegex(ValueError, 'serial group'):
            validate_metadata(row | {'category': 'unit', 'serial_group': 'shared-fixture'})

    def test_compiled_lists_reject_missing_and_unclassified_tests(self):
        rows = [dict(runner='rust', package='p', target_kind='lib', target='p', test_name='a')]
        verify_registered(rows, ['a'], ('p', 'lib', 'p'))
        for actual in ([], ['a', 'b']):
            with self.subTest(actual=actual), self.assertRaisesRegex(ValueError, 'inventory mismatch'):
                verify_registered(rows, actual, ('p', 'lib', 'p'))

    def test_same_test_name_in_another_target_does_not_satisfy_registration(self):
        rows = [dict(runner='rust', package='p', target_kind='bin', target='other', test_name='a')]
        with self.assertRaisesRegex(ValueError, 'unclassified'):
            verify_registered(rows, ['a'], ('p', 'bin', 'p'))

    def test_selection_keeps_behavior_category_separate_from_toolchain_requirements(self):
        ordinary = dict(id='ordinary', component='engine', category='integration', requirements=[])
        compiler = dict(id='compiler', component='engine', category='integration', requirements=['java'])
        other = dict(id='other', component='core', category='integration', requirements=[])
        rows = [ordinary, compiler, other]
        self.assertEqual(selected(rows, 'integration', 'engine', False), [ordinary])
        self.assertEqual(selected(rows, 'integration', 'engine', True), [ordinary, compiler])
        self.assertEqual(selected(rows, 'unit', None, True), [])
        self.assert_serial_execution_policy()
        self.assert_node_selection_isolation()

    def assert_node_selection_isolation(self):
        from verification.suite_execution import node_test_command
        rows = [dict(file=file, test_name=name)
                for file, names in (('first.test.js', ('shared', 'literal.+()')),
                                    ('second.test.js', ('shared', 'second only')))
                for name in names]
        command = node_test_command('first.test.js', rows)
        self.assertEqual(command[-1], 'first.test.js')
        pattern = command[-2]
        self.assertRegex('shared', pattern)
        self.assertRegex('literal.+()', pattern)
        self.assertNotRegex('second only', pattern)
        self.assertNotRegex('literal-other', pattern)

    def assert_serial_execution_policy(self):
        from verification.suite_execution import rust_group_key, rust_test_command
        unit = self.row(runner='rust', test_name='logic')
        integration = unit | {'category': 'integration', 'test_name': 'boundary'}
        acceptance = integration | {'category': 'acceptance'}
        serial = integration | {'serial_group': 'shared-fixture'}
        self.assertEqual(rust_group_key(integration), rust_group_key(acceptance))
        self.assertNotEqual(rust_group_key(integration), rust_group_key(serial))
        self.assertNotEqual(rust_group_key(serial), rust_group_key(serial | {'serial_group': 'other-fixture'}))
        self.assertNotEqual(rust_group_key(integration),
                            rust_group_key(integration | {'environment_append': {'MAVEN_OPTS': '-Dflag'}}))
        self.assertFalse(any(arg.startswith('--test-threads=') for arg in rust_test_command('tests', [unit])))
        self.assertIn('--test-threads=1', rust_test_command('tests', [integration]))
        self.assertIn('--test-threads=1', rust_test_command('tests', [acceptance]))
        self.assertIn('--test-threads=1', rust_test_command('tests', [serial]))
        ignored = integration | {'ignored': True}
        self.assertNotEqual(rust_group_key(integration), rust_group_key(ignored))
        self.assertNotIn('--ignored', rust_test_command('tests', [unit]))
        self.assertIn('--ignored', rust_test_command('tests', [ignored]))
        self.assertNotIn('--include-ignored', rust_test_command('tests', [ignored]))


class SourceRegistrationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        (self.root / 'src').mkdir()
        (self.root / 'Cargo.toml').write_text('[package]\nname="fixture"\nversion="0.1.0"\n')

    def write(self, path, text):
        destination = self.root / path
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(text)

    def test_inline_external_and_path_modules_have_exact_libtest_names(self):
        self.write('src/lib.rs', 'mod outer;\n#[path="special.rs"] mod renamed;')
        self.write('src/outer.rs', 'mod nested { #[tokio::test] async fn waits() {} }')
        self.write('src/special.rs', '#[test]\n#[ignore="real compiler"]\nfn check() {}')
        rows = discover_tests(self.root)
        self.assertEqual({row['test_name'] for row in rows}, {'outer::nested::waits', 'renamed::check'})
        self.assertTrue(next(row for row in rows if row['test_name'] == 'renamed::check')['ignored'])

    def test_source_inclusion_registers_separately_in_each_target(self):
        self.write('src/lib.rs', '#[test] fn shared() {}')
        self.write('tests/consumer.rs', '#[path="../src/lib.rs"] mod included;')
        rows = discover_tests(self.root)
        self.assertEqual(len(rows), 2)
        self.assertEqual({row['file'] for row in rows}, {'src/lib.rs'})
        self.assertEqual({row['target_kind'] for row in rows}, {'lib', 'test'})
        self.assertEqual(len({row['id'] for row in rows}), 2)

    def test_custom_binary_crate_root_resolves_sibling_modules(self):
        self.write('Cargo.toml', '[package]\nname="fixture"\nversion="0.1.0"\n[[bin]]\nname="alias"\npath="src/alias.rs"\n')
        self.write('src/main.rs', 'mod shared;')
        self.write('src/alias.rs', 'mod shared;')
        self.write('src/shared.rs', '#[test] fn policy() {}')
        rows = discover_tests(self.root)
        self.assertEqual({row['target'] for row in rows}, {'fixture', 'alias'})
        self.assertTrue(all(row['test_name'] == 'shared::policy' for row in rows))

    def test_python_mixins_register_only_under_concrete_test_case_classes(self):
        self.write('src/lib.rs', '')
        self.write('tests/test_sample.py', '''import unittest
class Cases:
    def test_shared(self): pass
class First(Cases, unittest.TestCase): pass
class Second(Cases, unittest.TestCase):
    def test_extra(self): pass
''')
        rows = discover_tests(self.root)
        self.assertEqual({row['test_name'] for row in rows},
                         {'test_sample.First.test_shared', 'test_sample.Second.test_shared',
                          'test_sample.Second.test_extra'})

    def test_node_modified_and_dynamic_registrations_fail_explicitly(self):
        self.write('src/lib.rs', '')
        for call in ("test.skip('name',()=>{});", "test.only('name',()=>{});",
                     "test.todo('name');", "test(variable,()=>{});"):
            with self.subTest(call=call):
                self.write('tools/typescript/sample.test.js', call)
                with self.assertRaisesRegex(ValueError, 'Node test'):
                    discover_tests(self.root)

    def test_node_literal_names_and_duplicate_detection_preserve_full_identity(self):
        self.write('src/lib.rs', '')
        self.write('tools/typescript/sample.test.js', "test('same name',()=>{});")
        rows = discover_tests(self.root)
        self.assertEqual(rows[0]['id'], 'tools/typescript/sample.test.js::same name')
        self.write('tools/typescript/sample.test.js', "test('same name',()=>{});\ntest('same name',()=>{});")
        with self.assertRaisesRegex(ValueError, 'Duplicate test'):
            discover_tests(self.root)

    def test_inventory_drift_and_explicit_additions_preserve_reviewed_metadata(self):
        self.write('src/lib.rs', '#[test] fn first() {}')
        rows = discover_tests(self.root)
        rows[0].update(component='core', category='unit', requirements=[], purpose='Existing reviewed purpose')
        self.write('tools/verification/test-inventory.json', json.dumps(dict(version=1, tests=rows, gates=[])))
        self.assertEqual(validate_inventory(self.root), rows)
        self.assert_gate_discovery_policy()
        self.write('src/lib.rs', '#[test] fn first() {}\n#[test] fn second() {}')
        with self.assertRaisesRegex(ValueError, 'unclassified'):
            validate_inventory(self.root)
        addition = Addition('src/lib.rs', 'core', 'integration', 'New boundary purpose')
        self.assertEqual(len(add_tests(self.root, addition)), 1)
        current = validate_inventory(self.root)
        self.assertEqual(current[0], rows[0])
        with self.assertRaisesRegex(ValueError, 'No new test'):
            add_tests(self.root, addition)
        self.write('src/lib.rs', '')
        with self.assertRaisesRegex(ValueError, 'stale'):
            validate_inventory(self.root)

    def assert_gate_discovery_policy(self):
        path = self.root / 'tools/verification/test-inventory.json'
        original = path.read_text()
        value = json.loads(original)
        files = ('tools/new_acceptance.py', 'tools/new_benchmark.py',
                 'tools/new_journeys.py', 'src/bin/query_benchmark.rs')
        ignored = 'tools/test_extra_acceptance.py'
        self.write(ignored, '')
        self.assertEqual(validate_inventory(self.root), value['tests'])
        for file in files:
            self.write(file, '')
            with self.assertRaisesRegex(ValueError, 'unclassified'):
                validate_inventory(self.root)
            value['gates'].append(dict(id='gate::' + file, runner='harness', file=file,
                                       test_name=Path(file).stem, component='tooling',
                                       category='acceptance', requirements=[],
                                       purpose='Reviewed real harness boundary', arguments=[]))
            path.write_text(json.dumps(value))
            self.assertEqual(validate_inventory(self.root), value['tests'])
        self.assert_gate_duplicate_and_stale(value, path)
        path.write_text(original)
        for file in (*files, ignored):
            (self.root / file).unlink()

    def assert_gate_duplicate_and_stale(self, value, path):
        duplicate = dict(value['gates'][0], id='different-id-same-harness')
        path.write_text(json.dumps(value | {'gates': [*value['gates'], duplicate]}))
        with self.assertRaisesRegex(ValueError, 'Duplicate standalone gate'):
            validate_inventory(self.root)
        path.write_text(json.dumps(value))
        file = self.root / value['gates'][0]['file']
        file.unlink()
        with self.assertRaisesRegex(ValueError, 'stale'):
            validate_inventory(self.root)
        file.write_text('')
