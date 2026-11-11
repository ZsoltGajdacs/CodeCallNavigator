"""Consumer/schema acceptance against retained compiler output and malformed variants."""
import copy
import json
import unittest

from jsonschema import ValidationError

from check_graph_contract import ROOT, consume, validator


class GraphContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.artifact = json.loads((ROOT / 'fixtures/.codecallnavigator/graph.json').read_bytes())
        cls.validator = validator()

    def mutated(self):
        return copy.deepcopy(self.artifact)

    def test_retained_real_compiler_graph_has_resolvable_edges_and_preserved_evidence(self):
        result = consume(self.artifact)
        self.assertGreater(len(result['symbols']), 0)
        self.assertGreater(len(result['relationships']), 0)
        self.assertEqual(result['diagnostics'], self.artifact['graph']['diagnostics'])
        self.assertEqual(
            sum(len(edge['evidence']) for edge in result['relationships']),
            sum(len(edge.get('evidence', [])) for edge in self.artifact['graph']['edges']),
        )
        self.assertTrue(all(edge['certainty'] for edge in result['relationships']))

    def test_formatted_and_compact_input_have_identical_consumer_results(self):
        compact = json.dumps(self.artifact, separators=(',', ':')) + '\n'
        formatted = json.dumps(self.artifact, indent=2) + '\n'
        self.assertEqual(consume(json.loads(compact)), consume(json.loads(formatted)))

    def test_additive_unknown_fields_do_not_change_known_consumer_values(self):
        artifact = self.mutated()
        extension = {'file': 'opaque', 'symbol': 'opaque', 'targets': ['z', 'a']}
        artifact['extension'] = extension
        artifact['graph']['extension'] = extension
        artifact['graph']['nodes'][0]['extension'] = extension
        artifact['graph']['edges'][0]['extension'] = extension
        self.validator.validate(artifact)
        self.assertEqual(consume(artifact)['relationships'], consume(self.artifact)['relationships'])
        self.assertEqual(artifact['extension']['targets'], ['z', 'a'])

    def test_invalid_known_fields_are_rejected_by_schema(self):
        for field, value in [('count', 0), ('count', '1'), ('from', 0), ('evidence', {})]:
            with self.subTest(field=field, value=value):
                artifact = self.mutated()
                artifact['graph']['edges'][0][field] = value
                with self.assertRaises(ValidationError):
                    self.validator.validate(artifact)
        artifact = self.mutated()
        artifact['schema'] = 'codecallnavigator.graph.v999'
        with self.assertRaises(ValidationError):
            self.validator.validate(artifact)

    def test_inventory_versions_and_flow_discriminators_are_closed(self):
        for section, payload in [
            ('flow_inventory', {'version': 999, 'functions': []}),
            ('architecture', {'version': 999, 'files': []}),
            ('boundaries', {'version': 999}),
            ('flow_inventory', {'version': 1, 'functions': [
                {'body': [{'kind': 'future_statement'}]},
            ]}),
        ]:
            with self.subTest(section=section, payload=payload):
                artifact = self.mutated()
                artifact['graph'][section] = payload
                with self.assertRaises(ValidationError):
                    self.validator.validate(artifact)

    def test_optional_analysis_metadata_has_a_versioned_known_shape(self):
        artifact = self.mutated()
        artifact['graph']['analysis'] = {
            'version': 1, 'targets': ['@repo/pom.xml'], 'roots': ['repo'],
            'engine_versions': 'fixture',
            'inputs': [{'path': '@repo/source.java', 'kind': 'file', 'hash': 'a' * 64},
                       {'path': '@repo/config', 'kind': 'missing', 'hash': ''}],
            'limitations': [{'code': 'unmapped_inputs', 'count': 1}],
        }
        self.validator.validate(artifact)
        for value in ['bad-hash', '', 'A' * 64]:
            with self.subTest(hash=value):
                artifact['graph']['analysis']['inputs'][0]['hash'] = value
                with self.assertRaises(ValidationError):
                    self.validator.validate(artifact)
        artifact['graph']['analysis']['inputs'][0]['hash'] = 'a' * 64
        artifact['graph']['analysis']['version'] = 999
        with self.assertRaises(ValidationError):
            self.validator.validate(artifact)

    def test_path_shape_is_checked_but_cross_references_need_semantic_validation(self):
        artifact = self.mutated()
        artifact['graph']['nodes'][0]['file'] = '/absolute/checkout/File.java'
        with self.assertRaises(ValidationError):
            self.validator.validate(artifact)
        artifact = self.mutated()
        artifact['graph']['edges'][0]['to'] = 'missing-symbol'
        self.validator.validate(artifact)
        with self.assertRaises(KeyError):
            consume(artifact)


if __name__ == '__main__':
    unittest.main()
