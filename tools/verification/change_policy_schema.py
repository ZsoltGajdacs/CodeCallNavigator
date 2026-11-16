"""Validate checked-in selection policy and inventory metadata at their boundary."""
from verification.inventory import COMPONENTS, unique_rows, validate_metadata


def require(condition, message):
    if not condition:
        raise ValueError(message)


def nonempty_strings(values):
    return isinstance(values, list) and bool(values) and all(isinstance(p, str) and p for p in values)


def validate_components(rule, name):
    values = rule.get(name, [])
    require(isinstance(values, list), 'Change rule components must be a list')
    require(all(c in COMPONENTS for c in values), 'Unknown component in change rule')


def validate_flags(rule):
    for name in ('full', 'static'):
        require(isinstance(rule.get(name, False), bool), 'Change rule flags must be booleans')


def validate_rule(rule):
    require(isinstance(rule, dict), 'Change rule must be an object')
    fields = {'id', 'patterns', 'full', 'correctness', 'performance', 'static'}
    require(not (set(rule) - fields), 'Invalid change rule fields')
    require(isinstance(rule.get('id'), str) and bool(rule['id']), 'Change rule requires an id')
    require(nonempty_strings(rule.get('patterns')), 'Change rule requires path patterns')
    validate_components(rule, 'correctness')
    validate_components(rule, 'performance')
    validate_flags(rule)
    require(any(rule.get(name) for name in fields - {'id', 'patterns'}), 'Change rule requires an action')


def validate_policy(value):
    require(isinstance(value, dict), 'Change policy must be an object')
    require(value.get('version') == 1, 'Unsupported change policy version')
    rules = value.get('rules')
    require(isinstance(rules, list) and bool(rules), 'Change policy requires rules')
    for rule in rules:
        validate_rule(rule)
    names = [rule['id'] for rule in rules]
    require(len(names) == len(set(names)), 'Duplicate change rule')
    return value


def inventory_rows(value):
    require(isinstance(value, dict), 'Inventory must be an object')
    require(value.get('version') == 1, 'Unsupported test inventory version')
    result = []
    for key in ('tests', 'gates'):
        rows = value.get(key)
        require(isinstance(rows, list), 'Inventory requires tests and gates arrays')
        for row in unique_rows(rows).values():
            validate_metadata(row)
            require(isinstance(row.get('file'), str), 'Inventory requires file paths')
        result.extend(rows)
    unique_rows(result)
    return result
