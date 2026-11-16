"""Pure file-policy behavior without compiler, filesystem or subprocess boundaries."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.change_inputs import Change, checked_path, parse_changes
from verification.change_policy import INVENTORY, inventory_delta, select_changes, validate_policy
from verification.change_plan import ChangePlan, report_scope, selected_identities


def row(identity, component='core', category='unit', file='tests/example.py'):
    return dict(id=identity, component=component, category=category, requirements=[],
                purpose='Observable policy outcome', file=file)


def inventory(*rows):
    return dict(version=1, tests=list(rows), gates=[])


def policy():
    return dict(version=1, rules=[
        dict(id='docs', patterns=['*.md'], static=True),
        dict(id='core', patterns=['crates/core/src/*'], correctness=['core', 'engine', 'ui', 'mcp']),
        dict(id='full', patterns=['vendor/*', 'Cargo.lock', 'tools/verification/*'], full=True),
        dict(id='performance', patterns=['crates/core/src/store*'], performance=['core']),
        dict(id='fixture', patterns=['fixtures/*'], correctness=['engine']),
    ])


class FilePolicyTests(unittest.TestCase):
    def test_unknown_path_requires_every_component_and_category(self):
        result = select_changes(['unknown.bin'], policy(), inventory())
        self.assertTrue(result.full)
        self.assertEqual(len(result.pairs), 20)
        self.assertEqual(result.matches[0]['rules'], ['unmatched-full'])

    def test_documentation_cannot_downgrade_fixture_or_full_rule(self):
        result = select_changes(['README.md', 'fixtures/example.md'], policy(), inventory())
        self.assertFalse(result.full)
        self.assertEqual(set(result.pairs), {(s, 'engine') for s in ('unit', 'integration', 'acceptance')})
        self.assertTrue(select_changes(['vendor/README.md'], policy(), inventory()).full)

    def test_core_consumers_and_explicit_performance_are_additive(self):
        result = select_changes(['crates/core/src/store.rs'], policy(), inventory())
        self.assertFalse(result.full)
        self.assertEqual(len(result.pairs), 13)
        self.assertIn(('acceptance', 'mcp'), result.pairs)
        self.assertIn(('performance', 'core'), result.pairs)
        plain = select_changes(['crates/core/src/model.rs'], policy(), inventory())
        self.assertNotIn(('performance', 'core'), plain.pairs)

    def test_order_duplicates_and_rule_order_do_not_change_selection(self):
        paths = ['README.md', 'crates/core/src/store.rs']
        forward = select_changes(paths, policy(), inventory())
        reversed_policy = policy()
        reversed_policy['rules'].reverse()
        self.assertEqual(forward, select_changes(paths[::-1] + paths, reversed_policy, inventory()))

    def test_registered_test_deletion_retains_old_owner(self):
        before = inventory(row('old', 'engine', 'integration'))
        result = select_changes(['tests/example.py'], policy(), inventory(), before)
        self.assertFalse(result.full)
        self.assertEqual(result.pairs, (('integration', 'engine'),))

    def test_production_file_with_tests_keeps_consumer_rule(self):
        current = inventory(row('test', file='crates/core/src/model.rs'))
        result = select_changes(['crates/core/src/model.rs'], policy(), current)
        self.assertEqual(len(result.pairs), 12)

    def test_inventory_additions_removals_and_reclassification_union_owners(self):
        before = inventory(row('changed'), row('removed', 'ui', 'integration'))
        after = inventory(row('changed', 'engine', 'acceptance'), row('added', 'mcp'))
        expected = [('acceptance', 'engine'), ('integration', 'ui'), ('unit', 'core'), ('unit', 'mcp')]
        self.assertEqual(inventory_delta(before, after), expected)
        result = select_changes([INVENTORY], dict(version=1, rules=[policy()['rules'][0]]), after, before)
        self.assertEqual(list(result.pairs), expected)
        self.assertFalse(result.full)

    def test_index_metadata_restored_in_worktree_keeps_staged_owner(self):
        base = inventory(row('same'))
        staged = inventory(row('same', 'engine', 'integration'))
        result = select_changes([INVENTORY], dict(version=1, rules=[policy()['rules'][0]]),
                                base, (base, staged))
        self.assertEqual(set(result.pairs), {('unit', 'core'), ('integration', 'engine')})

    def test_unchanged_inventory_and_empty_tree_have_empty_selection(self):
        current = inventory(row('same'))
        self.assertEqual(inventory_delta(current, copy.deepcopy(current)), [])
        result = select_changes([], policy(), current, current)
        self.assertFalse(result.full)
        self.assertEqual(result.pairs, ())

    def test_malformed_policy_and_inventory_fail_explicitly(self):
        for bad in ({}, dict(version=2, rules=[]), dict(version=1, rules=[{}])):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                validate_policy(bad)
        duplicate = policy()
        duplicate['rules'].append(duplicate['rules'][0])
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            validate_policy(duplicate)
        with self.assertRaises(ValueError):
            select_changes([], policy(), dict(version=1, tests=[], gates=None))

    def test_invalid_paths_cannot_escape_repository(self):
        for path in ('', '../outside.rs', '/tmp/source.rs', 'a/../b', 'nul\0file'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                checked_path(path)

    def test_name_status_keeps_both_rename_endpoints_and_unusual_names(self):
        changes = parse_changes(b'R100\0old name.rs\0new\nname.rs\0D\0deleted.rs\0', 'index')
        self.assertEqual(changes, [Change('old name.rs', 'R100', 'index'),
                                   Change('new\nname.rs', 'R100', 'index'),
                                   Change('deleted.rs', 'D', 'index')])
        with self.assertRaises(ValueError):
            parse_changes(b'R100\0old\0', 'index')

    def test_report_distinguishes_policy_exclusions_from_test_success(self):
        current = inventory(row('core'), row('ui', 'ui'))
        decision = select_changes(['tests/example.py'], policy(), current)
        report = selected_identities(current, decision)
        self.assertEqual(report['selected']['tests'], ['core', 'ui'])
        self.assertEqual(report['excluded_by_policy']['tests'], [])
        self.assertNotIn('passed', report)
        empty = select_changes([], policy(), current)
        context = ChangePlan(None, 'rev', [], policy(), empty, current)
        self.assertEqual(report_scope(context), 'empty')


if __name__ == '__main__':
    unittest.main()
