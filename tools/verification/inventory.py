"""Reviewed test ownership, behavioral categories and requirement selection.

The checked-in inventory is deliberately explicit. Newly discovered tests fail
validation until their owner records the boundary tested and its requirements.
"""
from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
import argparse
import json
from pathlib import Path
import re

from .inventory_discovery import discover_tests

COMPONENTS = ('ui', 'core', 'engine', 'mcp', 'tooling')
SUITES = ('unit', 'integration', 'acceptance', 'performance')
TOOLCHAINS = frozenset(('node', 'typescript', 'java', 'dotnet', 'private', 'native', 'profile'))
REQUIREMENTS = TOOLCHAINS | frozenset(('git', 'sqlite', 'graphviz', 'process-table', 'release',
                                     'maven', 'gradle', 'packaged', 'query-project', 'cli'))
SOURCE_FIELDS = ('id', 'runner', 'file', 'test_name', 'ignored', 'package', 'target_kind', 'target')
ENVIRONMENT_NAME = re.compile(r'[A-Za-z_][A-Za-z0-9_]*')


def inventory_path(root):
    return Path(root) / 'tools/verification/test-inventory.json'


def document(root):
    value = json.loads(inventory_path(root).read_text())
    if not isinstance(value, dict) or value.get('version') != 1:
        raise ValueError('Unsupported test inventory version')
    return value


def load_inventory(root):
    rows = document(root).get('tests')
    if not isinstance(rows, list):
        raise ValueError('Test inventory must contain a tests array')
    return rows


def load_gates(root):
    rows = document(root).get('gates')
    if not isinstance(rows, list):
        raise ValueError('Test inventory must contain a gates array')
    return rows


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(row):
    require(isinstance(row, dict), 'Every test or gate must be an object')
    value = row.get('id')
    require(isinstance(value, str), 'Every test or gate requires a string identity')
    return value


def unique_rows(rows):
    identities = [identity(row) for row in rows]
    repeated = [name for name, count in Counter(identities).items() if count > 1]
    require(not repeated, 'Duplicate inventory identities: ' + ', '.join(repeated))
    return dict(zip(identities, rows))


def validate_requirements(row):
    requirements = row.get('requirements')
    require(isinstance(requirements, list), f'Missing execution requirements: {row["id"]}')
    require(all(value in REQUIREMENTS for value in requirements), f'Unknown execution requirement: {row["id"]}')
    require(len(requirements) == len(set(requirements)), f'Duplicate execution requirement: {row["id"]}')
    if row['category'] == 'unit':
        require(not requirements, f'Unit test requires an external boundary: {row["id"]}')


def validate_metadata(row):
    require(row.get('component') in COMPONENTS, f'Unknown component/category: {row["id"]}')
    require(row.get('category') in SUITES, f'Unknown component/category: {row["id"]}')
    purpose = row.get('purpose')
    require(isinstance(purpose, str), f'Missing coverage purpose: {row["id"]}')
    require(bool(purpose.strip()), f'Missing coverage purpose: {row["id"]}')
    validate_requirements(row)
    settings = environment_append(row)
    require(not settings or row.get('runner') == 'rust',
            f'Fixture environment settings currently require the Rust runner: {row["id"]}')
    serial_group(row)


def serial_group(row):
    """Serialization records a demonstrated shared resource, never a test category."""
    if 'serial_group' not in row:
        return None
    value = row['serial_group']
    require(isinstance(value, str), f'Invalid serial group: {row["id"]}')
    require(bool(value.strip()), f'Invalid serial group: {row["id"]}')
    require(row['category'] != 'unit', f'Unit test cannot require a serial group: {row["id"]}')
    require(row.get('runner') == 'rust', f'A serial group currently requires the Rust runner: {row["id"]}')
    return value


def validate_environment_entry(name, value, identity):
    require(isinstance(name, str), f'Invalid environment name: {identity}')
    require(ENVIRONMENT_NAME.fullmatch(name), f'Invalid environment name: {identity}')
    require(isinstance(value, str), f'Invalid environment value: {identity}')
    require('\x00' not in value, f'Invalid environment value: {identity}')


def environment_append(row):
    """Per-test flags append to the child environment; absent metadata adds none."""
    values = row.get('environment_append', {})
    require(isinstance(values, dict), f'Invalid environment mapping: {row["id"]}')
    for name, value in values.items():
        validate_environment_entry(name, value, row['id'])
    return values


def validate_registration(row, discovered):
    validate_metadata(row)
    changed = any(row.get(key) != discovered.get(key) for key in SOURCE_FIELDS)
    require(not changed, f'Test registration changed: {row["id"]}')


def validate_gates(root, rows):
    gates = load_gates(root)
    unique_rows([*rows, *gates])
    files = gate_files(gates)
    discovered = discover_gates(root)
    missing, stale = sorted(discovered - files), sorted(files - discovered)
    require(not (missing or stale), f'Standalone gate inventory drift; unclassified={missing}; stale={stale}')
    for gate in gates:
        validate_metadata(gate)
        require(gate['category'] != 'unit', f'Standalone gate cannot be a unit test: {gate["id"]}')
        if 'requires_desktop' in gate:
            require(gate['requires_desktop'] is True and 'native' in gate['requirements'],
                    f'Invalid desktop requirement: {gate["id"]}')


def gate_files(gates):
    files = [gate.get('file') for gate in gates]
    require(all(isinstance(file, str) for file in files), 'Standalone gate requires a file path')
    repeated = sorted(file for file, count in Counter(files).items() if count > 1)
    require(not repeated, 'Duplicate standalone gate files: ' + ', '.join(repeated))
    return set(files)


def discover_gates(root):
    """Harness names register boundaries; category/requirements stay manually reviewed.

    Standalone Python harnesses use *_acceptance.py, *_benchmark.py or
    *_journeys.py directly under tools/ (excluding test_ modules). Rust benchmark
    entrypoints use src/bin/*benchmark*.rs. New matching files cannot silently
    escape full verification just because nobody added their inventory row.
    """
    root = Path(root)
    python = {path for pattern in ('*_acceptance.py', '*_benchmark.py', '*_journeys.py')
              for path in (root / 'tools').glob(pattern) if not path.name.startswith('test_')}
    rust = set((root / 'src/bin').glob('*benchmark*.rs'))
    return {str(path.relative_to(root)) for path in python | rust if path.is_file()}


def validate_inventory(root):
    rows = load_inventory(root)
    recorded = unique_rows(rows)
    discovered = unique_rows(discover_tests(root))
    missing, stale = sorted(discovered.keys() - recorded.keys()), sorted(recorded.keys() - discovered.keys())
    require(not (missing or stale), f'Test inventory drift; unclassified={missing}; stale={stale}')
    for key, row in recorded.items():
        validate_registration(row, discovered[key])
    validate_gates(root, rows)
    return rows


def component_names(component):
    """Normalize legacy single owners and explicit unions; None selects all owners."""
    if component is None:
        return COMPONENTS
    names = (component,) if isinstance(component, str) else component
    require(isinstance(names, (tuple, list)) and bool(names), 'Test component union must not be empty')
    require(all(isinstance(name, str) and name in COMPONENTS for name in names),
            f'Unknown test component: {component}')
    return tuple(sorted(set(names)))


@dataclass(frozen=True)
class Selection:
    suite: str
    component: str | tuple[str, ...] | None
    include_toolchains: bool

    def validate(self):
        require(self.suite in SUITES, f'Unknown test suite: {self.suite}')
        component_names(self.component)

    def matches(self, row):
        if row['category'] != self.suite:
            return False
        if row['component'] not in component_names(self.component):
            return False
        return self.include_toolchains or not TOOLCHAINS.intersection(row['requirements'])


def selected(rows, suite, component, include_toolchains):
    selection = Selection(suite, component, include_toolchains)
    selection.validate()
    return [row for row in rows if selection.matches(row)]


def select_tests(root, suite, component=None, include_toolchains=True):
    return selected(load_inventory(root), suite, component, include_toolchains)


def select_gates(root, suite, component=None, include_toolchains=True):
    return selected(load_gates(root), suite, component, include_toolchains)


def verify_registered(rows, registered, target):
    """Compare an executable's libtest listing with all its inventoried identities."""
    expected = {row['test_name'] for row in rows if row['runner'] == 'rust'
                and (row['package'], row['target_kind'], row['target']) == target}
    actual = set(registered)
    if expected != actual:
        raise ValueError(f'Compiled Rust inventory mismatch for {target}; '
                         f'unclassified={sorted(actual - expected)}; missing={sorted(expected - actual)}')


@dataclass(frozen=True)
class Addition:
    file: str
    component: str
    category: str
    purpose: str
    requirements: tuple = ()
    test_names: tuple = ()

    def matches(self, row):
        if row['file'] != self.file:
            return False
        return not self.test_names or row['test_name'] in self.test_names

    def annotate(self, row):
        row.update(component=self.component, category=self.category,
                   purpose=self.purpose, requirements=list(self.requirements))
        validate_metadata(row)


def add_tests(root, selection):
    """Add explicitly reviewed new registrations; never reclassify existing tests."""
    value = document(root)
    existing = unique_rows(value['tests'])
    additions = [row for row in discover_tests(root) if row['id'] not in existing and selection.matches(row)]
    require(bool(additions), f'No new test registrations in {selection.file}')
    for row in additions:
        selection.annotate(row)
    value['tests'] = sorted([*value['tests'], *additions], key=lambda row: row['id'])
    inventory_path(root).write_text(json.dumps(value, indent=2) + '\n')
    return additions


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--add', metavar='FILE', help='Add new registrations in this file with explicit metadata')
    parser.add_argument('--component', choices=COMPONENTS)
    parser.add_argument('--category', choices=SUITES)
    parser.add_argument('--purpose')
    parser.add_argument('--requirement', choices=sorted(REQUIREMENTS), action='append', default=[])
    parser.add_argument('--test-name', action='append', default=[], help='Exact test_name to add; repeat for mixed-category files')
    args = parser.parse_args()
    try:
        if args.add:
            if not all((args.component, args.category, args.purpose)):
                parser.error('--add requires --component, --category and --purpose')
            selection = Addition(args.add, args.component, args.category, args.purpose,
                                 tuple(args.requirement), tuple(args.test_name))
            add_tests(args.root, selection)
        rows = validate_inventory(args.root)
        counts = Counter((row['component'], row['category']) for row in rows)
        print(json.dumps(dict(tests=len(rows), gates=len(load_gates(args.root)),
                              counts={f'{component}/{category}': count
                                      for (component, category), count in sorted(counts.items())}), indent=2))
        return 0
    except (OSError, ValueError, KeyError) as error:
        print(f'Test inventory unavailable: {error}')
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
