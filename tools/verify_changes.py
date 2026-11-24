#!/usr/bin/env python3
"""Select required verification from changed files; execute only with --run."""
import argparse
import contextlib
from dataclasses import dataclass, replace
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
from verification.graphviz import locked_bin, prepend_locked_bin
from verification.inventory import validate_inventory
from verification.plan_output import display_plan
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
    phase: str = 'final'
    suite_preflight: bool = True


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default='HEAD', help='Direct comparison base, including all local changes')
    parser.add_argument('--format', choices=('text', 'json'), default='text')
    parser.add_argument('--phase', choices=('development', 'final'), default='final',
                        help='Select unit tests during development or the complete final plan (default)')
    action = parser.add_mutually_exclusive_group()
    action.add_argument('--run', action='store_true', help='Execute the plan for the selected phase')
    action.add_argument('--preflight', action='store_true', help='Check inputs for the selected plan without building')
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
    names = ('base', 'timeout', 'profile_source', 'output', 'codescene_command', 'keep_artifacts', 'phase')
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
    if settings.phase == 'development':
        # The development run may already own an explicitly requested evidence directory.
        report['final_execution_command'] = execution_command(replace(settings, phase='final', output=None))
    return report, steps


def execution_command(settings):
    command = [str(ROOT / 'verify-changes.sh'), '--run', '--base', settings.base, '--timeout', str(settings.timeout),
               '--codescene-command', settings.codescene_command]
    if settings.phase == 'development':
        command.extend(['--phase', settings.phase])
    for flag, value in (('--profile-source', settings.profile_source), ('--output', settings.output)):
        if value is not None:
            command.extend([flag, str(value)])
    if settings.keep_artifacts:
        command.append('--keep-artifacts')
    return shlex.join(command)


def configure_toolchain(report, steps):
    if 'graphviz' not in report['requirements']:
        return
    if pinned := locked_bin(ROOT):
        prepend_locked_bin(pinned, steps)


def preflight(report, steps, format_):
    step = next((step for step in steps if step.name.endswith('prerequisites')), None)
    result = check_inputs(report, step) if step else 0
    if format_ == 'json':
        display_plan(report | {'operation': 'preflight', 'result': RESULT_LABELS[result]}, format_)
    else:
        print(f'Verification preflight {RESULT_LABELS[result]}')
    return result


def check_inputs(report, step):
    configure_toolchain(report, [step])
    check = subprocess.run(step.command, cwd=ROOT, env=os.environ | step.environment,
                           stdin=subprocess.DEVNULL, stdout=sys.stderr, timeout=step.timeout)
    return int(check.returncode != 0)


def run(settings, report, steps, output):
    configure_toolchain(report, steps)
    with exclusive(ROOT):
        result = Runner(ROOT, output, steps).run()
        (output / 'change-plan.json').write_text(json.dumps(report, indent=2, sort_keys=True) + '\n')
        write_summary(ROOT, output, verification_phase=settings.phase,
                      tool_path=steps[0].environment.get('PATH'))
    return result


def main():
    args = arguments()
    settings = options(args)
    placeholder = settings.output or ROOT / 'target/verification/PLAN'
    report, steps = prepare(settings, placeholder)
    if args.preflight:
        return preflight(report, steps, args.format)
    if not args.run or not steps:
        if args.run and settings.phase == 'development':
            report['result'] = 'no-unit-tests'
        display_plan(report, args.format)
        return 0
    # Reserve an evidence name only after selection succeeds and --run was requested.
    output = legacy.evidence_output(settings)
    report, steps = prepare(settings, output)
    with contextlib.redirect_stdout(sys.stderr):
        result = run(settings, report, steps, output)
    report.update(result=RESULT_LABELS[result], evidence=str(output))
    display_plan(report, args.format)
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
