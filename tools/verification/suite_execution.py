"""Execute exact registered test selections; compilation is a separate stage."""
import argparse
import importlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from dataclasses import dataclass
from threading import Event
from functools import partial
from itertools import groupby
from urllib.parse import unquote, urlparse

from verification.inventory import environment_append, load_inventory, serial_group, validate_inventory, select_tests, verify_registered

BOUNDARY_TEST_THREADS = 4


def rust_build(root, output, rows, release):
    artifacts = []
    for package in sorted({row['package'] for row in rows}):
        selected = [row for row in rows if row['package'] == package]
        artifacts.extend(compile_package(root, rust_build_command(selected, release)))
    (output / 'rust-artifacts.json').write_text(json.dumps(artifacts, indent=2) + '\n')


def compile_package(root, command):
    artifacts = []
    process = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, text=True)
    for line in process.stdout:
        item = cargo_message(line)
        if item.get('reason') == 'compiler-artifact' and item.get('executable'):
            artifacts.append(item)
    if process.wait():
        raise RuntimeError('Rust test compilation failed')
    return artifacts


def rust_build_command(rows, release):
    # compile_package is called once per package so target flags cannot select
    # identically named tests in another package.
    command = ['cargo', 'test', '--locked', '--no-run', '--message-format=json', '-p', rows[0]['package']]
    if release:
        command.append('--release')
    for kind, target in sorted({(row['target_kind'], row['target']) for row in rows}):
        if kind == 'lib':
            command.append('--lib')
        else:
            command.extend(['--' + kind, target])
    return command


def cargo_message(line):
    try:
        item = json.loads(line)
    except ValueError:
        print(line, end='', flush=True)
        return {}
    if item.get('reason') == 'compiler-message':
        print(item['message'].get('rendered', ''), end='', flush=True)
    return item


def artifact_tests(artifact):
    result = subprocess.check_output([artifact['executable'], '--list', '--format=terse'], text=True)
    return {line[:-6] for line in result.splitlines() if line.endswith(': test')}


@dataclass(frozen=True)
class RustGroup:
    executable: str
    rows: list


def rust_run(root, output, rows, inventory):
    artifacts = json.loads((output / 'rust-artifacts.json').read_text())
    groups = []
    for artifact in artifacts:
        if artifact['profile'].get('test'):
            groups.extend(artifact_groups(root, artifact, rows, inventory))
    verify_rust_coverage(rows, groups)
    for batch in rust_batches(groups):
        run_rust_batch(root, batch)


def verify_rust_coverage(rows, groups):
    covered = {row['id'] for group in groups for row in group.rows}
    missing = {row['id'] for row in rows} - covered
    if missing:
        raise ValueError(f'Selected Rust tests have no executable: {sorted(missing)}')

def artifact_groups(root, artifact, rows, inventory):
    target = rust_target(root, artifact)
    verify_registered(inventory, artifact_tests(artifact), target)
    selected = [row for row in rows if row_target(row) == target]
    return [RustGroup(artifact['executable'], list(group))
            for _, group in groupby(sorted(selected, key=rust_group_key), key=rust_group_key)]


def exclusive_rust_group(row):
    return row['category'] in ('unit', 'performance') or 'native' in row.get('requirements', [])


def rust_batches(groups):
    pending = []
    for group in groups:
        if exclusive_rust_group(group.rows[0]):
            if pending:
                yield pending
                pending = []
            yield [group]
        else:
            pending.append(group)
    if pending:
        yield pending


def rust_chains(groups):
    chains = {}
    for index, group in enumerate(groups):
        key = ('serial', serial_group(group.rows[0])) if serial_group(group.rows[0]) else ('single', index)
        chains.setdefault(key, []).append((index, group))
    return iter(chains.values())


def run_rust_chain(root, chain, stopped):
    results = []
    try:
        for index, group in chain:
            if stopped.is_set():
                break
            result = run_rust_group(root, group.executable, group.rows)
            results.append((index, result))
            if result.returncode:
                stopped.set()
                break
        return results, None
    except BaseException as error:
        stopped.set()
        return results, error


def run_rust_batch(root, groups):
    stopped = Event()
    work = iter(partial(run_rust_chain, root, chain, stopped) for chain in rust_chains(groups))
    with ThreadPoolExecutor(max_workers=BOUNDARY_TEST_THREADS) as executor:
        results, errors = drain_rust_chains(executor, work, stopped)
    # Publish complete logs in invocation order after active children are reaped.
    for index in sorted(results):
        result = results[index]
        print(f'Rust test group: {result.args[0]}', flush=True)
        print(result.stdout, end='', flush=True)
    if errors:
        raise errors[0]
    for index in sorted(results):
        results[index].check_returncode()


def drain_rust_chains(executor, work, stopped):
    active = set()
    results, errors = {}, []
    submit_rust_chains(executor, work, stopped, active)
    try:
        while active:
            done, active = wait(active, return_when=FIRST_COMPLETED)
            errors.extend(collect_rust_results(done, results))
            if stopped.is_set():
                cancel_rust_chains(active)
            submit_rust_chains(executor, work, stopped, active)
    except BaseException:
        stopped.set()
        cancel_rust_chains(active)
        raise
    return results, errors


def collect_rust_results(done, results):
    errors = []
    for future in done:
        if future.cancelled():
            continue
        completed, error = future.result()
        results.update(completed)
        if error is not None:
            errors.append(error)
    return errors


def cancel_rust_chains(active):
    for future in active:
        future.cancel()


def submit_rust_chains(executor, work, stopped, active):
    while len(active) < BOUNDARY_TEST_THREADS and not stopped.is_set():
        task = next(work, None)
        if task is None:
            break
        active.add(executor.submit(task))


def environment_key(row):
    return json.dumps(environment_append(row), sort_keys=True)


def rust_group_key(row):
    return environment_key(row), serial_group(row) or '', bool(row.get('ignored')), exclusive_rust_group(row)


def appended_environment(row, base):
    result = dict(base)
    for name, suffix in environment_append(row).items():
        result[name] = (result.get(name, '') + ' ' + suffix).strip()
    return result


def run_rust_group(root, executable, rows):
    command = rust_test_command(executable, rows)
    try:
        return subprocess.run(command, cwd=root, env=appended_environment(rows[0], os.environ),
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              text=True, check=False)
    except OSError as error:
        return subprocess.CompletedProcess(command, 127, stdout=f'Cannot start Rust test group: {error}\n')


def rust_test_command(executable, rows):
    command = [executable, '--exact', '--nocapture']
    if rows[0].get('ignored'):
        command.append('--ignored')
    if serial_group(rows[0]) is not None:
        command.append('--test-threads=1')
    elif any(row['category'] != 'unit' for row in rows):
        # The outer scheduler owns the global four-group boundary budget.
        command.append('--test-threads=1')
    command.extend(row['test_name'] for row in rows)
    return command


def rust_target(root, artifact):
    package_path = artifact['package_id'].split('#')[0].removeprefix('path+')
    directory = Path(unquote(urlparse(package_path).path))
    package = 'codecallnavigator' if directory == root else directory.name
    target = artifact['target']
    return package, target['kind'][0], target['name']


def row_target(row):
    return row['package'], row['target_kind'], row['target']


def python_run(root, rows):
    # Test directories are not Python packages. Use unittest's real loader with
    # their module search paths, exactly as discovery does, but select exact IDs.
    sys.path[:0] = [str(root / 'tests'), str(root / 'tools')]
    import unittest
    verify_python_modules(rows, load_inventory(root))
    suite = unittest.TestLoader().loadTestsFromNames([row['test_name'] for row in rows])
    if suite.countTestCases() != len(rows):
        raise ValueError('Python registration count differs from the selected inventory')
    if not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful():
        raise RuntimeError('Python tests failed')


def python_case_ids(suite):
    import unittest
    if isinstance(suite, unittest.TestSuite):
        return set().union(*(python_case_ids(child) for child in suite))
    return {suite.id()}


def verify_python_modules(rows, inventory):
    import unittest
    for name in sorted({row['test_name'].split('.')[0] for row in rows}):
        module = importlib.import_module(name)
        actual = python_case_ids(unittest.TestLoader().loadTestsFromModule(module))
        expected = {row['test_name'] for row in inventory
                    if row['runner'] == 'python' and row['test_name'].split('.')[0] == name}
        if actual != expected:
            raise ValueError(f'Python registration drift in {name}: unclassified={actual-expected}; missing={expected-actual}')


def node_run(root, rows):
    files = sorted({row['file'] for row in rows})
    # Independent compiler fixtures may run together, but cap SDK/process pressure.
    # Capture each complete log so concurrent TAP streams cannot interleave.
    with ThreadPoolExecutor(max_workers=BOUNDARY_TEST_THREADS) as executor:
        results = list(executor.map(partial(node_file_run, root, rows), files))
    for result in results:
        print(f'Node test file: {result.args[-1]}', flush=True)
        print(result.stdout, end='', flush=True)
    for result in results:
        result.check_returncode()


def node_test_command(file, rows):
    names = [row['test_name'] for row in rows if row['file'] == file]
    pattern = '^(?:' + '|'.join(re.escape(name) for name in names) + ')$'
    return ['node', '--test', '--test-name-pattern', pattern, file]


def node_file_run(root, rows, file):
    command = node_test_command(file, rows)
    try:
        return subprocess.run(command, cwd=root, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT, text=True, check=False)
    except OSError as error:
        return subprocess.CompletedProcess(command, 127, stdout=f'Cannot start Node test file: {error}\n')


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['build', 'rust', 'python', 'node'])
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--suite', required=True)
    parser.add_argument('--component', action='append')
    parser.add_argument('--ordinary', action='store_true')
    return parser.parse_args()


def release_suite(args):
    if args.suite in ('acceptance', 'performance'):
        return True
    return args.suite == 'integration' and not args.ordinary


def main():
    args = arguments()
    inventory = validate_inventory(args.root)
    rows = select_tests(args.root, args.suite, args.component, not args.ordinary)
    runner = 'rust' if args.operation == 'build' else args.operation
    selected = [row for row in rows if row['runner'] == runner]
    if not selected:
        return
    operations = {
        'build': partial(rust_build, args.root, args.output, selected, release_suite(args)),
        'rust': partial(rust_run, args.root, args.output, selected, inventory),
        'python': partial(python_run, args.root, selected),
        'node': partial(node_run, args.root, selected),
    }
    operations[args.operation]()


if __name__ == '__main__':
    main()
