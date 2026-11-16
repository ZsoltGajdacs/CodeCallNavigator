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


def rust_check_steps(legacy, args, output, components):
    steps = legacy.check_steps(args, output)
    packages = sorted({PACKAGES[c] for c in components if c in PACKAGES})
    if not packages:
        return []
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


def focused_steps(legacy, context, output, paths):
    args = context.settings
    steps = static_steps(legacy.ROOT, paths, args.base, sys.executable)
    if any(path.endswith('.rs') for path in paths):
        components = {component for _, component in context.decision.pairs}
        steps.extend(rust_check_steps(legacy, args, output, components))
    steps.extend(category_plan(legacy, context, output))
    if any(Path(path).suffix.lower() in SOURCE_SUFFIXES for path in paths):
        steps.append(legacy.quality_step(args, paths))
    for step in steps:
        step.environment = legacy.environment(legacy.ROOT, output) | step.environment
    return steps


def build_steps(legacy, context, output):
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


def report_plan(context, steps):
    return dict(version=1, policy_version=context.policy['version'], base=context.settings.base,
                resolved_base=context.revision, scope=report_scope(context),
                changes=[dict(path=c.path, status=c.status, origin=c.origin) for c in context.changes],
                matches=list(context.decision.matches),
                selections=[dict(category=c, component=p) for c, p in context.decision.pairs],
                **selected_identities(context.inventory, context.decision),
                stages=[dict(name=s.name, command=s.command, environment=s.environment,
                             timeout=s.timeout, always=s.always) for s in steps])
