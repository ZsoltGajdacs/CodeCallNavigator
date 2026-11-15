"""Diagnostic timing and selection reports; elapsed time never determines success."""
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

CATEGORIES = ('unit', 'integration', 'acceptance', 'performance')
PHASES = ('compile', 'setup', 'test', 'cleanup', 'other')


def phase(name):
    operation = name
    for category in CATEGORIES:
        operation = operation.removeprefix(category + '-')
    if 'cleanup' in operation:
        return 'cleanup'
    if operation.startswith('compile-') or operation in ('cargo-check', 'clippy'):
        return 'compile'
    if operation.startswith(('test-', 'gate-')):
        return 'test'
    if operation.startswith(('prepare', 'own-', 'bootstrap', 'profile-snapshot', 'prerequisites')):
        return 'setup'
    return 'other'


def selection_counts(selection):
    selected, omitted = selection.get('selected', []), selection.get('not_run', [])
    gates, gates_omitted = selection.get('gates', []), selection.get('gates_not_run', [])
    components = {}
    for component in sorted({row['component'] for row in selected + omitted}):
        components[component] = dict(selected=sum(row['component'] == component for row in selected),
                                     not_run=sum(row['component'] == component for row in omitted))
    sources = {(row['file'], row['test_name']) for row in selected}
    return dict(selected_registrations=len(selected), selected_source_cases=len(sources),
                not_run_registrations=len(omitted), selected_gates=len(gates),
                not_run_gates=len(gates_omitted), components=components)


def recorded_costs(results):
    timings = dict.fromkeys(PHASES, 0.0)
    for result in results:
        timings[phase(result['name'])] += result.get('seconds', 0)
    return {key: round(value, 3) for key, value in timings.items()}


def category_summary(results, category, selection):
    stages = [row for row in results if row['name'].startswith(category + '-')]
    return selection_counts(selection) | dict(
        seconds=recorded_costs(stages),
        stage_statuses=dict(Counter(row['status'] for row in stages)))


def summarize_results(results, selections):
    """Aggregate reported stage costs without inventing per-test execution results."""
    timings = recorded_costs(results)
    categories = {category: category_summary(results, category, selection)
                  for category, selection in selections.items()}
    return dict(seconds=timings, recorded_stage_seconds=round(sum(timings.values()), 3),
                stage_statuses=dict(Counter(row['status'] for row in results)), categories=categories,
                timing_goal='Approximately one minute of warm unit execution is aspirational, never a pass/fail gate.',
                execution_note='Selection counts are registrations, not passing-test counts; failed/interrupted stages may execute only part of a selection.',
                comparison_note='Build/setup, execution and cleanup are separate. Cold/warm cache conditions are not inferred.')


def tool_version(command):
    executable = shutil.which(command[0])
    if executable is None:
        return dict(status='unavailable')
    try:
        result = subprocess.run([executable, *command[1:]], capture_output=True, text=True, timeout=5)
        lines = (result.stdout + result.stderr).strip().splitlines()
        return dict(status='available' if result.returncode == 0 else 'failed',
                    executable=executable, version=lines[:2], exit_code=result.returncode)
    except (OSError, subprocess.SubprocessError) as error:
        return dict(status='unavailable', executable=executable, reason=str(error))


def machine_details(include_tools=True):
    commands = {'cargo': ['cargo', '--version'], 'rustc': ['rustc', '--version'],
                'node': ['node', '--version'], 'java': ['java', '-version'],
                'dotnet': ['dotnet', '--version'], 'graphviz': ['dot', '-V']}
    return dict(os=platform.platform(), architecture=platform.machine(), logical_cpus=os.cpu_count(),
                python=dict(version=platform.python_version(), executable=sys.executable),
                tools={name: tool_version(command) for name, command in commands.items()} if include_tools else {})


def write_summary(root, output):
    """Write durable evidence after Runner, without changing its success criteria."""
    output = Path(output)
    results = json.loads((output / 'results.json').read_text())
    selections = {}
    for category in CATEGORIES:
        path = output / category / 'selection.json'
        if path.exists():
            selections[category] = json.loads(path.read_text())
    report = summarize_results(results, selections)
    runtime = bool(selections) or any(row['name'] == 'prerequisites' for row in results)
    report.update(version=1, generated_at=datetime.now(timezone.utc).isoformat(),
                  repository=str(Path(root).resolve()), machine=machine_details(runtime))
    (output / 'summary.json').write_text(json.dumps(report, indent=2) + '\n')
    return report
