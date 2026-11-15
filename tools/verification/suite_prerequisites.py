#!/usr/bin/env python3
"""Fail before compilation on prerequisites of the exact selected category."""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import shutil
import sys

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from verification import prerequisites as shared
from verification.inventory import select_gates, select_tests, validate_inventory, SUITES, COMPONENTS


def requirements_for(rows, gates):
    requirements = {requirement for row in [*rows, *gates] for requirement in row['requirements']}
    if 'packaged' in requirements:
        # The independently installed CLI package contains all three analyzer
        # resource sets, even when its consumer is an MCP-only journey.
        requirements.update(('java', 'dotnet', 'typescript'))
    if 'private' in requirements:
        requirements.update(('java', 'maven'))
    if 'java' in requirements:
        requirements.add('gradle')
    if 'typescript' in requirements:
        requirements.add('node')
    return requirements


def tools_for(requirements, runners):
    # Even a Python-only category creates an owned scratch workspace to clean.
    tools = {'/usr/sbin/lsof'}
    mapping = {'git': ('git',), 'node': ('node',), 'dotnet': ('dotnet',),
               'java': ('java', 'javac'), 'maven': ('mvn',), 'graphviz': ('dot',),
               'native': ('/usr/sbin/screencapture', '/usr/bin/osascript'),
               'packaged': ('codesign', 'otool', 'install_name_tool')}
    for requirement in requirements:
        tools.update(mapping.get(requirement, ()))
    if 'rust' in runners or requirements.intersection({'release', 'packaged', 'cli'}):
        tools.update(('cargo', 'rustc'))
    if 'node' in runners:
        tools.add('node')
    if 'rust' in runners or requirements.intersection({'release', 'packaged', 'cli', 'profile', 'native', 'process-table'}):
        tools.add('/usr/sbin/lsof')
    return tools


def node_supported(output):
    match = re.match(r'v?(\d+)\.', output.strip())
    return match is not None and int(match.group(1)) >= 24


def version_checks(root, requirements):
    specifications = {
        'node': (['node', '--version'], node_supported, 'Node 24+'),
        'dotnet': (['dotnet', '--list-sdks'], shared.dotnet_supported, '.NET SDK 11'),
        'java': (['java', '-version'], lambda text: 'version "25' in text, 'JDK 25'),
        'maven': (['mvn', '--version'], shared.maven_supported, 'Maven 3.9+/4'),
    }
    if 'graphviz' in requirements:
        expected = json.loads((root / 'analyzers.lock.json').read_text())['graphviz']['version']
        specifications['graphviz'] = (['dot', '-V'], lambda text: shared.graphviz_supported(text, expected), f'Graphviz {expected}')
    errors = []
    for requirement in sorted(requirements.intersection(specifications)):
        command, matches, hint = specifications[requirement]
        errors.extend(shared.checked(lambda: shared.require_version(command, matches, hint)))
    return errors


def private_problems(environment):
    private = environment.get('CODECALLNAVIGATOR_RECURRING_BILLING')
    if private and (Path(private) / 'pom.xml').is_file():
        return []
    return ['Set CODECALLNAVIGATOR_RECURRING_BILLING to the private Maven checkout with pom.xml']


def gradle_problems(root):
    if (root / 'fixtures/gradle/gradlew').is_file():
        return []
    return ['Missing fixtures/gradle/gradlew']


def configured_problems(root, requirements, profile_source, environment):
    """Check only selected resource boundaries; never demand unrelated private data."""
    checks = {'typescript': lambda: shared.typescript_problems(environment),
              'private': lambda: private_problems(environment),
              'gradle': lambda: gradle_problems(root),
              'profile': lambda: shared.profile_problems(profile_source)}
    return [error for requirement in sorted(requirements.intersection(checks))
            for error in shared.checked(checks[requirement])]


def native_problems(requirements):
    if 'native' not in requirements:
        return []
    if (platform.system(), platform.machine()) == ('Darwin', 'arm64'):
        return []
    return ['Native verification requires Apple Silicon macOS with Metal/display access; no screen permission is requested by preflight']


def problems(root, rows, gates, profile_source=None):
    root = Path(root)
    requirements = requirements_for(rows, gates)
    runners = {row['runner'] for row in rows}
    if 'node' in runners:
        requirements.add('node')
    errors = [f'Missing executable: {tool}' for tool in sorted(tools_for(requirements, runners)) if not shutil.which(tool)]
    errors.extend(shared.checked(lambda: version_checks(root, requirements)))
    errors.extend(shared.checked(lambda: configured_problems(root, requirements, profile_source, os.environ)))
    errors.extend(native_problems(requirements))
    if 'python' in runners:
        errors.extend(shared.checked(lambda: shared.python_problems(root)))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--suite', choices=SUITES, required=True)
    parser.add_argument('--component', choices=COMPONENTS)
    parser.add_argument('--ordinary', action='store_true')
    parser.add_argument('--profile-source', type=Path, default=os.environ.get('CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE'))
    args = parser.parse_args()
    try:
        validate_inventory(args.root)
        rows = select_tests(args.root, args.suite, args.component, not args.ordinary)
        gates = select_gates(args.root, args.suite, args.component, not args.ordinary)
        errors = problems(args.root, rows, gates, args.profile_source)
    except (OSError, ValueError, KeyError) as error:
        errors = [str(error)]
    if errors:
        print('Selected suite prerequisites are incomplete:\n- ' + '\n- '.join(errors))
        return 1
    print('Selected suite requirements are available; native screen permissions are not proven by preflight.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
