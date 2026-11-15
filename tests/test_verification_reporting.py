"""Pure report aggregation keeps costs and incomplete execution honest."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from verification.reporting import summarize_results


class ReportingTests(unittest.TestCase):
    def test_phase_costs_keep_failed_and_cleanup_time_visible(self):
        results = [dict(name='unit-compile-rust', status='passed', seconds=61),
                   dict(name='unit-prepare', status='passed', seconds=2),
                   dict(name='unit-test-rust', status='failed', seconds=75),
                   dict(name='unit-category-cleanup', status='passed', seconds=3)]
        report = summarize_results(results, {})
        self.assertEqual(report['seconds'], dict(compile=61, setup=2, test=75, cleanup=3, other=0))
        self.assertEqual(report['recorded_stage_seconds'], 141)
        self.assertEqual(report['stage_statuses'], dict(passed=3, failed=1))
        self.assertNotIn('passed', report)
        self.assertNotIn('within_budget', report)

    def test_selection_counts_separate_selected_from_omitted(self):
        selected = [dict(file='a.rs', test_name='one', component='core')]
        omitted = [dict(file='b.rs', test_name='two', component='core')]
        report = summarize_results([], {'unit': dict(selected=selected, not_run=omitted,
                                                    gates=[dict(id='gate')], gates_not_run=[dict(id='other')])})
        category = report['categories']['unit']
        self.assertEqual(category['selected_registrations'], 1)
        self.assertEqual(category['not_run_registrations'], 1)
        self.assertEqual(category['selected_gates'], 1)
        self.assertEqual(category['not_run_gates'], 1)
        self.assertEqual(category['components']['core'], dict(selected=1, not_run=1))

    def test_incomplete_execution_never_becomes_passing_test_count(self):
        stages = [dict(name='integration-test-rust', status='interrupted', seconds=4),
                  dict(name='integration-test-python', status='not-run')]
        selected = [dict(file='a.rs', test_name='one', component='engine')]
        report = summarize_results(stages, {'integration': dict(selected=selected)})
        category = report['categories']['integration']
        self.assertEqual(category['stage_statuses'], {'interrupted': 1, 'not-run': 1})
        self.assertEqual(category['seconds']['test'], 4)
        self.assertEqual(category['selected_registrations'], 1)
        self.assertNotIn('passed_tests', category)

    def test_components_and_source_definitions_are_counted_separately(self):
        selected = [dict(file='a.rs', test_name='one', component='ui', target='bin'),
                    dict(file='a.rs', test_name='one', component='ui', target='lib'),
                    dict(file='b.rs', test_name='two', component='core', target='lib')]
        report = summarize_results([], {'unit': dict(selected=selected)})
        category = report['categories']['unit']
        self.assertEqual(category['selected_registrations'], 3)
        self.assertEqual(category['selected_source_cases'], 2)
        self.assertEqual(category['components']['ui']['selected'], 2)
        self.assertEqual(category['components']['core']['selected'], 1)


if __name__ == '__main__':
    unittest.main()
