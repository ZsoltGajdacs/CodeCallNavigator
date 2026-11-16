"""Pure, additive file policy: unmatched paths require full verification."""
from dataclasses import dataclass
from fnmatch import fnmatchcase
import json

from verification.change_inputs import checked_path
from verification.inventory import COMPONENTS, SUITES
from verification.change_policy_schema import inventory_rows, validate_policy

INVENTORY = 'tools/verification/test-inventory.json'
CORRECTNESS = ('unit', 'integration', 'acceptance')


@dataclass(frozen=True)
class Decision:
    full: bool
    pairs: tuple[tuple[str, str], ...]
    matches: tuple[dict, ...]


def differing_registrations(old, new):
    changed = {key for key in old.keys() | new.keys() if old.get(key) != new.get(key)}
    return [row for rows in (old, new) for key, row in rows.items() if key in changed]


def inventory_delta(before, after):
    old = {row['id']: row for row in inventory_rows(before)} if before is not None else {}
    new = {row['id']: row for row in inventory_rows(after)}
    return sorted({(row['category'], row['component']) for row in differing_registrations(old, new)})


def rule_pairs(rule):
    return {(category, component) for component in rule.get('correctness', []) for category in CORRECTNESS} | {
        ('performance', component) for component in rule.get('performance', [])}


def registered_pairs(path, rows):
    # Production modules may contain tests: their file rule remains authoritative.
    dedicated = path.startswith('tests/') or '/tests/' in path or path.startswith('tools/')
    if not dedicated:
        return set()
    return {(row['category'], row['component']) for row in rows if row['file'] == path}


def matching_rules(path, policy):
    return [rule for rule in policy['rules'] if any(fnmatchcase(path, p) for p in rule['patterns'])]


def path_selection(path, policy, rows, delta):
    checked_path(path)
    rules = matching_rules(path, policy)
    pairs = registered_pairs(path, rows)
    reasons = [rule['id'] for rule in rules]
    if pairs:
        reasons.append('registered-test-or-gate')
    if path == INVENTORY:
        pairs.update(delta)
        reasons.append('inventory-delta')
    for rule in rules:
        pairs.update(rule_pairs(rule))
    full = any(rule.get('full', False) for rule in rules)
    if not reasons:
        return True, pairs, dict(path=path, rules=['unmatched-full'])
    return full, pairs, dict(path=path, rules=sorted(reasons))


def inventory_history(previous_inventory, inventory):
    history = previous_inventory if isinstance(previous_inventory, tuple) else (previous_inventory,)
    previous, delta = [], set()
    for snapshot in history:
        if snapshot is not None:
            previous.extend(inventory_rows(snapshot))
        delta.update(inventory_delta(snapshot, inventory))
    return previous, delta


def selection_union(selections):
    pairs = set()
    for _, selected, _ in selections:
        pairs.update(selected)
    return pairs


def select_changes(paths, policy, inventory, previous_inventory=None):
    validate_policy(policy)
    rows = inventory_rows(inventory)
    previous, delta = inventory_history(previous_inventory, inventory)
    selections = [path_selection(path, policy, rows + previous, delta) for path in sorted(set(paths))]
    full = any(selection[0] for selection in selections)
    pairs = selection_union(selections)
    if full:
        pairs = {(category, component) for category in SUITES for component in COMPONENTS}
    return Decision(full, tuple(sorted(pairs)), tuple(s[2] for s in selections))


def read_json(path):
    return json.loads(path.read_bytes())
