#!/usr/bin/env python3
"""Validate/consume portable artifacts independently of CodeGraph Rust models.

Requires the test-only dependency jsonschema>=4.18. Accepts retained fixtures or
fresh production output, e.g. check_graph_contract.py /checkout/.codegraph/graph.json.
"""
import argparse
from collections import Counter
import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parent.parent


def validator():
    schema = json.loads((ROOT / 'docs/graph.schema.json').read_text())
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def consume(artifact):
    """Resolve edge IDs and retain certainty, evidence and diagnostics as a consumer would."""
    validator().validate(artifact)
    graph = artifact['graph']
    symbols = {node['id']: node for node in graph['nodes']}
    if len(symbols) != len(graph['nodes']):
        raise ValueError('Duplicate symbol identity')
    relationships = []
    for edge in graph['edges']:
        relationships.append({
            'caller': symbols[edge['from']]['label'],
            'target': symbols[edge['to']]['label'],
            'kind': edge['kind'],
            'certainty': edge['certainty'],
            'count': edge['count'],
            'provenance': edge.get('provenance', ''),
            'evidence': edge.get('evidence', []),
        })
    return {
        'symbols': symbols,
        'relationships': relationships,
        'diagnostics': graph['diagnostics'],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('artifacts', nargs='+', type=Path)
    arguments = parser.parse_args()
    for path in arguments.artifacts:
        result = consume(json.loads(path.read_bytes()))
        print(json.dumps({
            'artifact': str(path), 'nodes': len(result['symbols']),
            'edges': len(result['relationships']),
            'certainty': dict(Counter(edge['certainty'] for edge in result['relationships'])),
            'evidence_records': sum(len(edge['evidence']) for edge in result['relationships']),
            'diagnostics': len(result['diagnostics']),
        }))


if __name__ == '__main__':
    main()
