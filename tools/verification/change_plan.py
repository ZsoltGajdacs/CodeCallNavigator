"""Compose deterministic policy selections through the existing stage builders."""
from dataclasses import dataclass, replace
import sys

from verification.checks import static_steps
from pathlib import Path
from verification.inventory import SUITES
from verification.quality import SOURCE_SUFFIXES
from verification.suite_prerequisites import requirements_for
from verification.suites import category_steps

PACKAGES = {'ui': 'codecallnavigator', 'core': 'codecallnavigator-core',
            'engine': 'codecallnavigator-engine', 'mcp': 'codecallnavigator-mcp'}


def selected_rows(inventory, decision):
    pairs = set(decision.pairs)
    return {key: sorted((row for row in inventory[key] if (row['category'], row['component']) in pairs),
                        key=lambda row: row['id']) for key in ('tests', 'gates')}


def rust_check_steps(legacy, args, output, packages):
    steps = legacy.check_steps(args, output)
    for step in steps:
        if step.name in ('cargo-check', 'clippy'):
            at = step.command.index('--workspace')
            step.command[at:at + 1] = [part for package in packages for part in ('-p', package)]
    return steps


@dataclass(frozen=True)
class ChangePlan:
    settings: object
    revision: str
    changes: list
    policy: dict
    decision: object
    inventory: dict

    @property
    def paths(self):
        return sorted({change.path for change in self.changes})


def category_plan(legacy, context, output):
    selected = selected_rows(context.inventory, context.decision)
    steps = []
    for category in SUITES:
        components = tuple(sorted({row['component'] for rows in selected.values() for row in rows
                                   if row['category'] == category}))
        if components:
            steps.extend(category_steps(legacy.ROOT, category, output,
                                        replace(context.settings, component=components)))
    return steps


def rust_packages(context, paths):
    if not any(path.endswith('.rs') for path in paths):
        return []
    return sorted({PACKAGES[component] for _, component in context.decision.pairs
                   if component in PACKAGES})


def needs_quality(paths):
    return any(Path(path).suffix.lower() in SOURCE_SUFFIXES for path in paths)


def focused_steps(legacy, context, output, paths):
    args = context.settings
    steps = static_steps(legacy.ROOT, paths, args.base, sys.executable)
    packages = rust_packages(context, paths)
    if packages:
        steps.extend(rust_check_steps(legacy, args, output, packages))
    steps.extend(category_plan(legacy, context, output))
    if needs_quality(paths):
        steps.append(legacy.quality_step(args, paths))
    for step in steps:
        step.environment = legacy.environment(legacy.ROOT, output) | step.environment
    return steps


def development_decision(context):
    unit_components = {row['component'] for row in context.inventory['tests']
                       if row['category'] == 'unit'}
    pairs = tuple(pair for pair in context.decision.pairs
                  if pair[0] == 'unit' and pair[1] in unit_components)
    return replace(context.decision, full=False, pairs=pairs)


def development_steps(legacy, context, output):
    settings = replace(context.settings, profile_source=None)
    active = replace(context, decision=development_decision(context), settings=settings)
    steps = category_plan(legacy, active, output)
    for step in steps:
        step.environment = legacy.environment(legacy.ROOT, output) | step.environment
    return steps


def build_steps(legacy, context, output):
    if context.settings.phase == 'development':
        return development_steps(legacy, context, output)
    # Vendor edits select full, but upstream files must never be formatted.
    paths = [path for path in context.paths if not path.startswith('vendor/')]
    if context.decision.full:
        return legacy.plan(replace(context.settings, mode='full'), paths, output)
    if not context.paths:
        return []
    return focused_steps(legacy, context, output, paths)


def report_scope(context):
    if context.decision.full:
        return 'full'
    if context.decision.pairs:
        return 'focused'
    return 'static' if context.changes else 'empty'


def row_ids(rows):
    return sorted(row['id'] for row in rows)


def excluded_ids(rows, identities):
    chosen = set(identities)
    return row_ids(row for row in rows if row['id'] not in chosen)


def selected_identities(inventory, decision):
    selected = selected_rows(inventory, decision)
    ids = {key: row_ids(rows) for key, rows in selected.items()}
    excluded = {key: excluded_ids(inventory[key], ids[key]) for key in ids}
    return dict(selected=ids, excluded_by_policy=excluded,
                requirements=sorted(requirements_for(selected['tests'], selected['gates'])))


def final_check_groups(context):
    if not context.changes:
        return []
    if context.decision.full:
        return ['static', 'full prerequisites and bootstrap', 'Rust checks', 'CodeScene', 'full cleanup']
    groups = ['static']
    paths = context.paths
    if rust_packages(context, paths):
        groups.append('Rust checks')
    if needs_quality(paths):
        groups.append('CodeScene')
    return groups


def development_selection(context):
    active = development_decision(context)
    planned = selected_identities(context.inventory, context.decision)
    actual = selected_identities(context.inventory, active)
    deferred = {kind: sorted(set(identities) - set(actual['selected'][kind]))
                for kind, identities in planned['selected'].items()}
    deferred['checks'] = final_check_groups(context)
    labels = dict(phase='development', deferred_until_final=deferred)
    return active.pairs, actual | dict(excluded_by_policy=planned['excluded_by_policy']) | labels


def report_plan(context, steps):
    pairs, selection = (development_selection(context) if context.settings.phase == 'development'
                        else (context.decision.pairs, selected_identities(context.inventory, context.decision)))
    return dict(version=1, policy_version=context.policy['version'], base=context.settings.base,
                resolved_base=context.revision, scope=report_scope(context),
                changes=[dict(path=c.path, status=c.status, origin=c.origin) for c in context.changes],
                matches=list(context.decision.matches),
                selections=[dict(category=c, component=p) for c, p in pairs],
                **selection,
                stages=[dict(name=s.name, command=s.command, environment=s.environment,
                             timeout=s.timeout, always=s.always,
                             requires_desktop=s.requires_desktop) for s in steps])
