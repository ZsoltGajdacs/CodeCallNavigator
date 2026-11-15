"""Literal Node test identities; unsupported registration must fail visibly."""
import re

MODIFIED_TEST = re.compile(r'^\s*(?:test\.(?:skip|only|todo)|it|describe|t\.test)\s*\(', re.MULTILINE)
TEST_CALL = re.compile(r'^\s*test\(', re.MULTILINE)
TEST_NAME = re.compile(r'''^\s*test\(\s*(['"])(.*?)\1\s*,''', re.MULTILINE)


def node_names(path):
    source = path.read_text()
    if MODIFIED_TEST.search(source):
        raise ValueError(f'Unsupported Node test registration; use literal top-level test calls: {path}')
    calls, names = TEST_CALL.findall(source), TEST_NAME.findall(source)
    if len(calls) != len(names):
        raise ValueError(f'Node tests require literal unique names: {path}')
    return [name for _, name in names]


def node_file(root, path):
    for name in node_names(path):
        yield dict(id=f'{path.relative_to(root)}::{name}', runner='node',
                   file=str(path.relative_to(root)), test_name=name, ignored=False)


def node_tests(root):
    for path in sorted((root / 'tools/typescript').glob('*.test.js')):
        yield from node_file(root, path)
