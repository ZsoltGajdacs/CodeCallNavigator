"""Performance comparisons must detect semantic changes even at identical scale."""
import json
from pathlib import Path
import sqlite3
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from analysis_graph import (
    compare, compare_snapshots, graph_snapshot, snapshot_from_facts,
    volatile_generated_hashes, volatile_hashes_from_snapshots,
)


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
        edge = graph_snapshot(self.current)['edges'][0]
        self.assertEqual((edge['from'], edge['to'], edge['provenance']),
                         ('caller', 'target', 'compiler'))
        self.assertEqual(volatile_generated_hashes([self.reference, self.current]), set())

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


class GraphComparisonPolicyTest(unittest.TestCase):
    def metadata(self, modified_ns=100, **overrides):
        return {
            'diagnostics': ['known limitation'], 'roots': ['/fixture'],
            'method_policy': 2, 'engine_versions': 'pinned',
            'fingerprints': [{'path': '/fixture/source', 'hash': 'content', 'size': 10,
                              'modified_ns': modified_ns}],
        } | overrides

    def snapshot(self, metadata, *, generated=False):
        nodes = [{'id': 'caller', 'file': '/fixture/source', 'is_generated': generated}]
        return snapshot_from_facts(nodes, [], metadata)

    def test_flow_and_architecture_payloads_are_compared(self):
        before = self.snapshot(self.metadata())
        after = self.snapshot(self.metadata(flow_inventory={'version': 1, 'functions': []},
                                            architecture={'version': 1, 'files': []}))
        self.assertEqual(compare_snapshots(before, after)['different_fields'],
                         ['architecture', 'flow_inventory'])

    def test_boundary_inventory_changes_are_compared(self):
        def with_methods(methods):
            return self.snapshot(self.metadata(boundaries={
                'version': 1, 'routes': [{'path': '/items/{id}', 'methods': methods,
                                          'restrictions': ['condition']}], 'uses': [],
            }))

        after = with_methods(['POST', 'GET'])
        self.assertEqual(compare_snapshots(self.snapshot(self.metadata()), after)['different_fields'],
                         ['boundaries'])
        self.assertTrue(compare_snapshots(with_methods(['GET', 'POST']), after)['equal'])

    def test_dropping_freshness_inputs_is_detected_but_observation_times_are_ignored(self):
        before = self.snapshot(self.metadata())
        self.assertTrue(compare_snapshots(before, self.snapshot(self.metadata(modified_ns=200)))['equal'])
        after = self.snapshot(self.metadata(fingerprints=[]))
        self.assertEqual(compare_snapshots(before, after)['different_fields'], ['fingerprints'])

    def test_baseline_volatility_applies_only_to_generated_hashes_and_preserves_sizes(self):
        changed = self.metadata()
        changed['fingerprints'][0]['hash'] = 'another generator timestamp'
        before, after = self.snapshot(self.metadata()), self.snapshot(changed)
        self.assertEqual(volatile_hashes_from_snapshots([before, after]), set())
        self.assertFalse(compare_snapshots(before, after)['equal'])

        before = self.snapshot(self.metadata(), generated=True)
        after = self.snapshot(changed, generated=True)
        allowed = volatile_hashes_from_snapshots([before, after])
        self.assertEqual(allowed, {'/fixture/source'})
        result = compare_snapshots(before, after, allowed)
        self.assertTrue(result['equal'])
        self.assertEqual(result['volatile_generated_hash_changes'], ['/fixture/source'])
        changed['fingerprints'][0]['size'] = 999
        self.assertFalse(compare_snapshots(before, self.snapshot(changed, generated=True), allowed)['equal'])


if __name__ == '__main__':
    unittest.main()
