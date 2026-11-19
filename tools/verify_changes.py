#!/usr/bin/env python3
"""Select required verification from changed files; execute only with --run."""
import argparse
import contextlib
from dataclasses import dataclass
import json
import os
from pathlib import Path
import shlex
import signal
import subprocess
import sys

import verify as legacy
from verification.change_inputs import discover_changes, inventory_at
from verification.change_policy import INVENTORY, read_json, select_changes, validate_policy
from verification.change_plan import ChangePlan, build_steps, report_plan
from verification.inventory import validate_inventory
from verification.reporting import write_summary
from verification.runner import RESULT_LABELS, Runner, exclusive

ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class Options:
    base: str = 'HEAD'
    timeout: int = 1800
    profile_source: Path | None = None
    output: Path | None = None
    codescene_command: str = 'cs-mcp'
    keep_artifacts: bool = False
    mode: str = 'changes'
    component: tuple[str, ...] | None = None


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default='HEAD', help='Direct comparison base, including all local changes')
    parser.add_argument('--format', choices=('text', 'json'), default='text')
    parser.add_argument('--run', action='store_true', help='Execute the complete computed plan')
    parser.add_argument('--timeout', type=int, default=1800)
    parser.add_argument('--profile-source', type=Path, default=os.environ.get('CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE'))
    parser.add_argument('--output', type=Path)
    parser.add_argument('--codescene-command', default='cs-mcp')
    parser.add_argument('--keep-artifacts', action='store_true')
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error('--timeout must be positive')
    return args


def options(args):
    names = ('base', 'timeout', 'profile_source', 'output', 'codescene_command', 'keep_artifacts')
    values = {name: getattr(args, name) for name in names}
    for name in ('profile_source', 'output'):
        if values[name] is not None:
            values[name] = values[name].resolve()
    if os.path.sep in values['codescene_command']:
        values['codescene_command'] = str(Path(values['codescene_command']).resolve())
    return Options(**values)


def prepare(settings, output):
    revision, changes = discover_changes(ROOT, settings.base)
    paths = sorted({change.path for change in changes})
    policy = validate_policy(read_json(ROOT / 'tools/verification/change-rules.json'))
    inventory = read_json(ROOT / INVENTORY)
    snapshots = [inventory_at(ROOT, ref, INVENTORY) for ref in (revision, ':')]
    previous = tuple(json.loads(value) if value is not None else None for value in snapshots)
    decision = select_changes(paths, policy, inventory, previous)
    if decision.pairs:
        validate_inventory(ROOT)
    context = ChangePlan(settings, revision, changes, policy, decision, inventory)
    steps = build_steps(legacy, context, output)
    report = report_plan(context, steps)
    report['execution_command'] = execution_command(settings)
    return report, steps


def execution_command(settings):
    command = [str(ROOT / 'verify-changes.sh'), '--run', '--base', settings.base, '--timeout', str(settings.timeout),
               '--codescene-command', settings.codescene_command]
    for flag, value in (('--profile-source', settings.profile_source), ('--output', settings.output)):
        if value is not None:
            command.extend([flag, str(value)])
    if settings.keep_artifacts:
        command.append('--keep-artifacts')
    return shlex.join(command)


def display(report, form):
    if form == 'json':
        print(json.dumps(report, indent=2, sort_keys=True))
        return
    print(f"Verification: {report['scope']} (base {report['base']} = {report['resolved_base']})")
    for match in report['matches']:
        print(f"  {match['path']!r}: {', '.join(match['rules'])}")
    for selection in report['selections']:
        print(f"  Required: {selection['category']} / {selection['component']}")
    print('Requirements: ' + (', '.join(report['requirements']) or 'none'))
    for stage in report['stages']:
        desktop = " (requires accessible desktop)" if stage['requires_desktop'] else ""
        print(f"  {stage['name']}{desktop}: {shlex.join(stage['command'])}")
    print('Execute: ' + report['execution_command'])
    if 'result' in report:
        print(f"Result: {report['result']}; evidence: {report['evidence']}")


def run(settings, report, steps, output):
    with exclusive(ROOT):
        result = Runner(ROOT, output, steps).run()
        (output / 'change-plan.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
        write_summary(ROOT, output)
    return result


def main():
    args = arguments()
    settings = options(args)
    placeholder = settings.output or ROOT / 'target/verification/PLAN'
    report, steps = prepare(settings, placeholder)
    if not args.run or not steps:
        display(report, args.format)
        return 0
    # Reserve an evidence name only after selection succeeds and --run was requested.
    output = legacy.evidence_output(settings)
    report, steps = prepare(settings, output)
    with contextlib.redirect_stdout(sys.stderr):
        result = run(settings, report, steps, output)
    report.update(result=RESULT_LABELS[result], evidence=str(output))
    display(report, args.format)
    return result


if __name__ == '__main__':
    signal.signal(signal.SIGTERM, legacy.interrupt)
    try:
        status = main()
    except KeyboardInterrupt:
        print('Verification interrupted', file=sys.stderr)
        status = 130
    except (OSError, RuntimeError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f'Verification unavailable: {error}', file=sys.stderr)
        status = 2
    raise SystemExit(status)
