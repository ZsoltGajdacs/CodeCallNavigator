"""The extraction oracle must compare native and persisted representations fairly."""
from pathlib import Path
import subprocess
import sys
import unittest

TOOLS = Path(__file__).resolve().parents[1] / 'tools'
sys.path.insert(0, str(TOOLS))
from check_engine_reference import from_native


class EngineReferenceTest(unittest.TestCase):
    def test_native_normalization_preserves_edges_and_optional_node_attributes(self):
        graph = {
            'nodes': [{'id': 'z', 'visibility': 'public'}, {'id': 'a'}],
            'edges': [{'from': 0, 'to': 1, 'kind': 'call', 'count': 2,
                       'certainty': 'direct', 'provenance': 'compiler'}],
            'fingerprints': [{'path': '/source', 'hash': 'content', 'size': 10,
                              'modified_ns': 100}],
        }
        snapshot = from_native(graph)
        self.assertEqual(snapshot['nodes'], [
            {'id': 'a', 'visibility': None, 'container_visibility': None},
            {'id': 'z', 'visibility': 'public', 'container_visibility': None},
        ])
        self.assertEqual(snapshot['edges'], [{
            'from': 'z', 'to': 'a', 'kind': 'call', 'count': 2,
            'certainty': 'direct', 'provenance': 'compiler',
        }])
        self.assertEqual(snapshot['fingerprints'], [{
            'path': '/source', 'hash': 'content', 'size': 10,
            'missing': False, 'excluded_children': [],
        }])

    def test_unknown_workload_is_an_error_instead_of_an_empty_successful_run(self):
        result = subprocess.run([
            sys.executable, str(TOOLS / 'check_engine_reference.py'),
            '--reference', '/missing-reference', '--workload', 'not-a-workload',
        ], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn('invalid choice', result.stderr)
        self.assertEqual(result.stdout, '')


if __name__ == '__main__':
    unittest.main()
