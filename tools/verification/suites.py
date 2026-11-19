"""Shared category plans used by standalone scripts and light/full composites."""
from dataclasses import dataclass
import os
from pathlib import Path
import sys

from verification.inventory import validate_inventory, select_tests, select_gates, load_gates, component_names
from verification.runner import Step
from verification.category_cleanup import temporary_path
from verification.suite_prerequisites import requirements_for

CATEGORIES = ('unit', 'integration', 'acceptance', 'performance')
COMPONENTS = ('ui', 'core', 'engine', 'mcp', 'tooling')
BUILD_SERVER_ENVIRONMENT = {'DOTNET_CLI_USE_MSBUILD_SERVER': '0',
                            'MSBUILDDISABLENODEREUSE': '1',
                            'UseSharedCompilation': 'false'}


@dataclass(frozen=True)
class SuiteSelection:
    category: str
    component: str | tuple[str, ...] | None = None
    ordinary: bool = False
    timeout: int = 1800
    keep_artifacts: bool = False

    @classmethod
    def from_args(cls, category, args):
        ordinary = getattr(args, 'mode', None) == 'light' or getattr(args, 'ordinary', False)
        component = getattr(args, 'component', None)
        if component is not None and not isinstance(component, str):
            component = component_names(component)
        return cls(category, component, ordinary,
                   getattr(args, 'timeout', 1800), getattr(args, 'keep_artifacts', False))

    @property
    def profile(self):
        release = self.category in ('acceptance', 'performance')
        release = release or self.category == 'integration' and not self.ordinary
        return 'release' if release else 'debug'

    def arguments(self):
        command = ['--suite', self.category]
        if self.component is not None:
            for component in component_names(self.component):
                command.extend(['--component', component])
        if self.ordinary:
            command.append('--ordinary')
        return command


@dataclass(frozen=True)
class SuiteContext:
    root: Path
    output: Path
    selection: SuiteSelection

    @property
    def build(self):
        return self.output / 'build'

    def environment(self):
        engine = str(self.build / self.selection.profile / 'codecallnavigator-engine')
        store = 'profile' if self.selection.category == 'performance' else 'test-store'
        return BUILD_SERVER_ENVIRONMENT | {'CARGO_TARGET_DIR': str(self.build), 'CARGO_BUILD_BUILD_DIR': str(self.build),
                'CARGO_INCREMENTAL': '0', 'CARGO_CACHE_AUTO_CLEAN_FREQUENCY': 'never',
                'TMPDIR': str(temporary_path(self.root, self.output)),
                'CODECALLNAVIGATOR_GRAPH_LIBRARY': str(temporary_path(self.root, self.output) / 'graphs'),
                'PYTHONPATH': os.pathsep.join([str(self.root / 'tools'), os.environ.get('PYTHONPATH', '')]),
                'CODECALLNAVIGATOR_DATA_DIR': str(self.output / store),
                'CODECALLNAVIGATOR_ENGINE': engine, 'CODECALLNAVIGATOR_TEST_ENGINE': engine,
                'CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE': engine}

    def selected_tests(self):
        choice = self.selection
        return select_tests(self.root, choice.category, choice.component, not choice.ordinary)

    def selected_gates(self):
        choice = self.selection
        return select_gates(self.root, choice.category, choice.component, not choice.ordinary)


def category_steps(root, category, output, args):
    """Flatten leaf stages into one Runner: one owner reaps all child groups."""
    context = SuiteContext(root, output / category, SuiteSelection.from_args(category, args))
    steps, report = execution_steps(context)
    result = [preparation_step(context)]
    if getattr(args, 'mode', None) != 'full':
        result.insert(0, prerequisite_step(context, args))
    requirements = requirements_for(report['selected'], report['gates'])
    if needs_bootstrap(context, requirements, args):
        result.append(bootstrap_step(context, requirements))
    setup = gate_setup_steps(context, report['gates'], args)
    for step in setup:
        step.environment = gate_environment(context) | step.environment
    result.extend(setup)
    steps[-1:-1] = gate_steps(context, report['gates'], args)
    result.extend(steps)
    for step in result:
        step.name = f'{category}-{step.name}'
    return result


def prerequisite_step(context, args):
    command = [sys.executable, str(context.root / 'tools/verification/suite_prerequisites.py'),
               '--root', str(context.root), *context.selection.arguments()]
    if getattr(args, 'profile_source', None):
        command.extend(['--profile-source', str(args.profile_source)])
    return Step('prerequisites', command, 180)


def bootstrap_step(context, requirements):
    command = [sys.executable, str(context.root / 'tools/bootstrap.py')]
    languages = requirements & {'java', 'typescript'}
    if 'dotnet' in requirements:
        languages = languages | {'csharp'}
    for language in sorted(languages):
        command.extend(['--language', language])
    return Step('bootstrap', command, context.selection.timeout)


def needs_bootstrap(context, requirements, args):
    if getattr(args, 'mode', None) == 'full':
        return False  # The full composite has already bootstrapped once.
    return not context.selection.ordinary and bool(requirements & {'typescript', 'java', 'dotnet'})


def preparation_step(context):
    choice = context.selection
    command = [sys.executable, str(context.root / 'tools/prepare_test_suite.py'),
               choice.category, '--output', str(context.output)]
    command.extend(choice.arguments()[2:])
    return Step('prepare', command)


def execution_steps(context):
    inventory = validate_inventory(context.root)
    rows = context.selected_tests()
    steps = test_operation_steps(context, rows)
    steps.append(category_cleanup_step(context))
    return steps, selection_report(context, inventory, rows)


def test_operation_steps(context, rows):
    steps = []
    if any('cli' in row['requirements'] for row in rows):
        steps.append(cli_compilation_step(context))
    for operation in ('build', 'rust', 'python', 'node'):
        runner = 'rust' if operation == 'build' else operation
        if any(row['runner'] == runner for row in rows):
            steps.append(operation_step(context, operation))
    return steps


def cli_compilation_step(context):
    command = ['cargo', 'build', '--locked', '-p', 'codecallnavigator-engine', '--bins']
    if context.selection.profile == 'release':
        command.append('--release')
    return Step('compile-cli', command, context.selection.timeout, context.environment())


def operation_step(context, operation):
    command = [sys.executable, '-m', 'verification.suite_execution', operation,
               '--root', str(context.root), '--output', str(context.output),
               *context.selection.arguments()]
    name = 'compile-rust' if operation == 'build' else f'test-{operation}'
    return Step(name, command, context.selection.timeout, context.environment())


def category_cleanup_step(context):
    command = [sys.executable, str(context.root / 'tools/verification/category_cleanup.py'),
               '--root', str(context.root), '--output', str(context.output), '--if-unprepared']
    if context.selection.keep_artifacts:
        command.append('--keep-artifacts')
    return Step('category-cleanup', command, 600, always=True)


def selection_report(context, inventory, rows):
    choice = context.selection
    gates = context.selected_gates()
    return {'category': choice.category, 'component': choice.component, 'selected': rows,
            'components': list(component_names(choice.component)),
            'not_run': [],
            'excluded_by_policy': excluded_rows(inventory, choice.category, rows),
            'timing_goal': 'Approximately one minute of warm unit execution is aspirational, not a gate.',
            'gates': gates,
            'gates_not_run': [],
            'gates_excluded_by_policy': excluded_rows(load_gates(context.root), choice.category, gates)}


def excluded_rows(inventory, category, selected):
    """Selection omissions are policy exclusions, never failed execution evidence."""
    identities = {row['id'] for row in selected}
    return [row for row in inventory if row['category'] == category and row['id'] not in identities]


def gate_environment(context):
    return BUILD_SERVER_ENVIRONMENT | {'CARGO_TARGET_DIR': str(context.build), 'CARGO_BUILD_BUILD_DIR': str(context.build),
            'TMPDIR': str(temporary_path(context.root, context.output)),
            'CODECALLNAVIGATOR_GRAPH_LIBRARY': str(temporary_path(context.root, context.output) / 'graphs'),
            'CODECALLNAVIGATOR_DATA_DIR': str(context.output / 'profile'),
            'PATH': os.pathsep.join([str(context.root / 'dist/CodeCallNavigatorEngine/bin'),
                                    str(context.root / 'dist/CodeCallNavigatorMcp/bin'),
                                    os.environ.get('PATH', '')])}


def gate_steps(context, gates, args):
    environment = gate_environment(context)
    steps = []
    bindings = gate_bindings(context)
    for gate in gates:
        steps.extend(gate_execution_steps(context, gate, bindings))
    for step in steps:
        step.environment = environment | step.environment
    return steps


def gate_setup_steps(context, gates, args):
    steps = []
    if any('packaged' in gate['requirements'] for gate in gates):
        steps.extend(package_steps(context, args))
    if any(gate['file'] == 'tools/query_benchmark.py' for gate in gates):
        command = ['cargo', 'build', '--locked', '--release', '-p', 'codecallnavigator-engine', '--bins']
        steps.append(Step('compile-query-cli', command, args.timeout))
        fixture = 'seed_query_fixture'
        command = ['cargo', 'build', '--locked', '--release', '-p', 'codecallnavigator-core', '--example', fixture]
        steps.append(Step('compile-query-fixture-loader', command, args.timeout))
        project = context.root / 'fixtures'
        command = [str(context.build / 'release/examples' / fixture), str(project),
                   str(project / '.codecallnavigator/graph.json')]
        steps.append(Step('seed-query-fixture', command, args.timeout))
    if any('profile' in row['requirements'] for row in [*gates, *context.selected_tests()]):
        source = getattr(args, 'profile_source', None) or context.output / 'MISSING_PROFILE_SOURCE'
        command = [sys.executable, str(context.root / 'tools/verification/profile.py'),
                   str(source), str(context.output / 'profile')]
        steps.append(Step('profile-snapshot', command, args.timeout))
    return steps


def gate_bindings(context):
    root = context.root
    return {'root': str(root), 'engine': str(context.build / 'release/codecallnavigator-engine'),
            'app': str(root / 'dist/CodeCallNavigator.app'),
            'modules': os.environ.get('CODECALLNAVIGATOR_TS_NODE_MODULES', 'MISSING_TYPESCRIPT_MODULES'),
            'engine_package': str(root / 'dist/CodeCallNavigatorEngine'),
            'mcp': str(root / 'dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp'),
            'project': str(root / 'fixtures')}


def gate_execution_steps(context, gate, bindings):
    bindings = bindings | {'output': str(context.output / ('gate-' + gate['test_name']))}
    steps = []
    if gate['file'] == 'src/bin/benchmark.rs':
        command = ['cargo', 'build', '--locked', '--release', '--bin', 'benchmark']
        steps.append(Step('compile-storage-benchmark', command, context.selection.timeout))
        command = [str(context.build / 'release/benchmark'), str(context.output / 'scale-cache')]
    else:
        command = [sys.executable, str(context.root / gate['file']),
                   *[argument.format_map(bindings) for argument in gate['arguments']]]
    steps.append(Step('gate-' + gate['test_name'], command, context.selection.timeout,
                      requires_desktop=gate.get('requires_desktop', False)))
    return steps


def package_steps(context, args):
    root = context.root
    gates = context.selected_gates()
    desktop = any('native' in gate['requirements'] for gate in gates)
    command = [sys.executable, str(root / 'tools/package.py')]
    if not desktop:
        command.append('--engine-only')
    steps = [Step('package', command, args.timeout)]
    if desktop:
        steps.append(Step('package-signature', ['codesign', '--verify', '--deep', '--strict', str(root / 'dist/CodeCallNavigator.app')], args.timeout))
    steps.append(Step('packaged-engine', [str(root / 'dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine'), '--help']))
    steps.extend(integration_package_steps(context, gates, args))
    return steps


def integration_package_steps(context, gates, args):
    from verification.checks import agent_integration_steps
    root = context.root
    steps = []
    if any('agent_' in gate['test_name'] for gate in gates):
        steps.extend(step for step in agent_integration_steps(root, args, context.output) if step.name != 'agent-cli-journeys')
    if any(gate['component'] == 'mcp' for gate in gates):
        steps.append(Step('mcp-independent-install', [sys.executable, str(root / 'tools/package_mcp.py'), '--engine', str(root / 'dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine')], args.timeout))
    return steps
