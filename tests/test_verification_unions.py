"""Pure component unions preserve coverage without duplicating category execution."""
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.inventory import COMPONENTS, component_names, selected
from verification.reporting import selection_counts, summarize_results
from verification.suite_prerequisites import requirements_for
from verification.suites import (
    SuiteContext, SuiteSelection, excluded_rows, operation_step, preparation_step,
    prerequisite_step, test_operation_steps,
)


def registration(identity, component, category='unit', requirements=()):
    return dict(id=identity, component=component, category=category,
                requirements=list(requirements), runner='rust', file=identity + '.rs',
                test_name=identity)


class ComponentUnionTests(unittest.TestCase):
    def test_union_selects_every_owner_once_and_keeps_inventory_order(self):
        rows = [registration('ui', 'ui'), registration('core', 'core'),
                registration('engine', 'engine')]
        selection = ('engine', 'core', 'engine')
        self.assertEqual(selected(rows, 'unit', selection, True), rows[1:])
        self.assertEqual(selected(rows, 'unit', 'core', True), rows[1:2])
        self.assertEqual(selected(rows, 'unit', None, True), rows)
        self.assertEqual(component_names(None), COMPONENTS)

    def test_empty_unknown_and_invalid_component_unions_fail(self):
        for names in ((), [], ('core', 'unknown'), ('core', None), 1, {'core'}):
            with self.subTest(names=names), self.assertRaises(ValueError):
                selected([], 'unit', names, True)

    def test_union_keeps_category_and_toolchain_requirements(self):
        rows = [registration('core', 'core', 'integration'),
                registration('java', 'engine', 'integration', ('java',)),
                registration('acceptance', 'engine', 'acceptance')]
        self.assertEqual(selected(rows, 'integration', ('core', 'engine'), True), rows[:2])
        self.assertEqual(selected(rows, 'integration', ('core', 'engine'), False), rows[:1])
        self.assertEqual(requirements_for(rows[:2], []), {'java', 'gradle'})

    def test_child_commands_carry_the_same_canonical_union(self):
        args = SimpleNamespace(component=['engine', 'core', 'engine'], mode='changes', timeout=17)
        choice = SuiteSelection.from_args('integration', args)
        self.assertEqual(choice.component, ('core', 'engine'))
        self.assertEqual(choice.profile, 'release')
        context = SuiteContext(Path('/repo'), Path('/evidence/integration'), choice)
        expected = ['--component', 'core', '--component', 'engine']
        commands = [preparation_step(context).command, operation_step(context, 'build').command,
                    prerequisite_step(context, args).command]
        for command in commands:
            with self.subTest(command=command):
                self.assertEqual(command[-4:], expected)
        self.assertEqual(SuiteSelection('unit', 'core').arguments(),
                         ['--suite', 'unit', '--component', 'core'])

    def test_union_produces_one_compile_and_execution_stage_per_runner(self):
        rows = [registration('core', 'core'), registration('engine', 'engine')]
        context = SuiteContext(Path('/repo'), Path('/evidence/unit'),
                               SuiteSelection('unit', ('core', 'engine')))
        steps = test_operation_steps(context, rows)
        self.assertEqual([step.name for step in steps], ['compile-rust', 'test-rust'])
        for step in steps:
            self.assertEqual(step.command.count('--component'), 2)


class PolicyExclusionTests(unittest.TestCase):
    def test_policy_exclusions_are_separate_from_selected_execution_omissions(self):
        rows = [registration('core', 'core'), registration('ui', 'ui'),
                registration('engine', 'engine', 'integration')]
        chosen = selected(rows, 'unit', 'core', True)
        excluded = excluded_rows(rows, 'unit', chosen)
        self.assertEqual(excluded, rows[1:2])
        selection = dict(selected=chosen, excluded_by_policy=excluded,
                         gates_excluded_by_policy=[dict(id='gate', component='ui')])
        report = selection_counts(selection)
        self.assertEqual(report['selected_registrations'], 1)
        self.assertEqual(report['not_run_registrations'], 0)
        self.assertEqual(report['excluded_by_policy_registrations'], 1)
        self.assertEqual(report['excluded_by_policy_gates'], 1)
        self.assertEqual(report['excluded_by_policy_components'], {'ui': 1})
        result = summarize_results([dict(name='unit-test-rust', status='not-run')],
                                   {'unit': selection})
        category = result['categories']['unit']
        self.assertEqual(category['stage_statuses'], {'not-run': 1})
        self.assertEqual(category['selected_registrations'], 1)
        self.assertEqual(category['excluded_by_policy_registrations'], 1)
        self.assertNotIn('passed_tests', category)


if __name__ == '__main__':
    unittest.main()
