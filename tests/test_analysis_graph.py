"""Performance comparisons must detect semantic changes even at identical scale."""
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from analysis_graph import compare, volatile_generated_hashes


class GraphComparisonTest(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.TemporaryDirectory()
        self.addCleanup(self.root.cleanup)
        self.reference = Path(self.root.name) / 'before'
        self.current = Path(self.root.name) / 'after'
        self.create(self.reference, False)
        self.create(self.current, True)

    def create(self, cache, reverse):
        cache.mkdir()
        with sqlite3.connect(cache / 'catalog.sqlite3') as connection:
            connection.execute('CREATE TABLE catalog (key TEXT, value TEXT)')
            connection.execute("INSERT INTO catalog VALUES ('active','graph.sqlite3')")
        with sqlite3.connect(cache / 'graph.sqlite3') as connection:
            connection.executescript('''
                CREATE TABLE nodes (idx INTEGER, id TEXT, label TEXT, signature TEXT, line INTEGER, is_generated INTEGER, file TEXT);
                CREATE TABLE edges (idx INTEGER, from_idx INTEGER, to_idx INTEGER, kind TEXT, certainty TEXT, count INTEGER);
                CREATE TABLE metadata (key TEXT, value TEXT);
            ''')
            indices = (1, 0) if reverse else (0, 1)
            connection.executemany('INSERT INTO nodes VALUES (?,?,?,?,?,?,?)',
                                   [(indices[0], 'caller', 'start', 'start()', 10, 0, '/fixture/source'),
                                    (indices[1], 'target', 'leaf', 'leaf()', 20, 0, '/fixture/target')])
            connection.execute('INSERT INTO edges VALUES (?,?,?,?,?,?)',
                               (0, *indices, 'call', 'direct', 2))
            metadata = {'diagnostics': ['known limitation'], 'roots': ['/fixture'],
                        'entry_points': {'candidates': ['caller'], 'application_starts': []},
                        'method_policy': 2, 'engine_versions': 'pinned', 'edge_provenance': ['compiler'],
                        'fingerprints': [{'path': '/fixture/source', 'hash': 'content', 'size': 10,
                                          'modified_ns': 200 if reverse else 100}]}
            connection.execute("INSERT INTO metadata VALUES ('graph',?)", (json.dumps(metadata),))

    def test_node_numbering_does_not_change_equivalence(self):
        result = compare(self.reference, self.current)
        self.assertTrue(result['equal'])
        self.assertEqual(result['baseline_sha256'], result['current_sha256'])
        self.assertEqual((result['nodes'], result['edges']), (2, 1))

    def test_changed_label_or_edge_count_is_detected_without_scale_change(self):
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            connection.execute("UPDATE nodes SET label='wrong lambda label' WHERE id='caller'")
            connection.execute('UPDATE edges SET count=3')
        result = compare(self.reference, self.current)
        self.assertFalse(result['equal'])
        self.assertEqual(result['different_fields'], ['nodes', 'edges'])
        self.assertEqual((result['nodes'], result['edges']), (2, 1))

    def test_provenance_and_diagnostics_are_compared(self):
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            metadata = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
            metadata['edge_provenance'] = ['heuristic']
            metadata['diagnostics'] = []
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))
        self.assertEqual(compare(self.reference, self.current)['different_fields'], ['edges', 'diagnostics'])

    def test_flow_and_architecture_payloads_are_compared(self):
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            metadata = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
            metadata['flow_inventory'] = {'version': 1, 'functions': []}
            metadata['architecture'] = {'version': 1, 'files': []}
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))
        self.assertEqual(compare(self.reference, self.current)['different_fields'],
                         ['architecture', 'flow_inventory'])

    def test_relationship_evidence_changes_are_compared_but_empty_legacy_evidence_is_not_added(self):
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            metadata = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
            metadata['edge_evidence'] = [[]]
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))
        self.assertTrue(compare(self.reference, self.current)['equal'])
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            metadata['edge_evidence'] = [[{'rule': 'react.event', 'version': 1}]]
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))
        self.assertEqual(compare(self.reference, self.current)['different_fields'], ['edges'])

    def test_dropping_freshness_inputs_is_detected_but_observation_times_are_ignored(self):
        self.assertTrue(compare(self.reference, self.current)['equal'])
        with sqlite3.connect(self.current / 'graph.sqlite3') as connection:
            metadata = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
            metadata['fingerprints'] = []
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))
        self.assertEqual(compare(self.reference, self.current)['different_fields'], ['fingerprints'])

    def change_fingerprint(self, cache, values):
        with sqlite3.connect(cache / 'graph.sqlite3') as connection:
            metadata = json.loads(connection.execute('SELECT value FROM metadata').fetchone()[0])
            metadata['fingerprints'][0].update(values)
            connection.execute('UPDATE metadata SET value=?', (json.dumps(metadata),))

    def test_baseline_volatility_applies_only_to_generated_hashes_and_preserves_sizes(self):
        self.change_fingerprint(self.current, {'hash': 'another generator timestamp'})
        self.assertEqual(volatile_generated_hashes([self.reference, self.current]), set())
        self.assertFalse(compare(self.reference, self.current)['equal'])
        for cache in [self.reference, self.current]:
            with sqlite3.connect(cache / 'graph.sqlite3') as connection:
                connection.execute("UPDATE nodes SET is_generated=1 WHERE id='caller'")
        allowed = volatile_generated_hashes([self.reference, self.current])
        self.assertEqual(allowed, {'/fixture/source'})
        result = compare(self.reference, self.current, allowed)
        self.assertTrue(result['equal'])
        self.assertEqual(result['volatile_generated_hash_changes'], ['/fixture/source'])
        self.change_fingerprint(self.current, {'size': 999})
        self.assertFalse(compare(self.reference, self.current, allowed)['equal'])


if __name__ == '__main__':
    unittest.main()
