"""Change selection and deterministic verification stage construction."""
import os
import subprocess
import sys
from pathlib import Path
from verification.runner import Step


def changed_files(root, base):
    def git(*arguments):
        return subprocess.check_output(["git", *arguments], cwd=root).decode().split("\0")
    paths = git("diff", "--name-only", "-z", base, "--")
    paths += git("ls-files", "--others", "--exclude-standard", "-z")
    return sorted({p for p in paths if p and not p.startswith("vendor/")})


def docs_only(paths):
    return all(not p.startswith("fixtures/") and (
        Path(p).suffix == ".md" or p.startswith(".backlog/")
        or Path(p).name in {"LICENSE", "NOTICE", "COPYING"}) for p in paths)


def syntax_command(path, python):
    commands = {
        '.rs': ['rustfmt', '--check', '--edition', '2024', '--config', 'skip_children=true'],
        '.py': [python, '-c', 'import ast, pathlib, sys; ast.parse(pathlib.Path(sys.argv[1]).read_bytes(), filename=sys.argv[1])'],
        '.json': [python, '-c', 'import json, pathlib, sys; json.loads(pathlib.Path(sys.argv[1]).read_bytes())'],
        '.sh': ['bash', '-n'],
        '.js': ['node', '--check'], '.cjs': ['node', '--check'], '.mjs': ['node', '--check'],
    }
    command = commands.get(Path(path).suffix)
    return [*command, path] if command else None


def static_steps(root, paths, base, python):
    steps = [Step('whitespace', ['git', 'diff', '--check', base, '--'])]
    for index, path in enumerate(paths):
        if not (root / path).is_file():
            continue
        command = syntax_command(path, python)
        if command:
            steps.append(Step(f'syntax-format-{index:03d}', command))
    return steps


def agent_integration_steps(root, args, output):
    python = sys.executable
    package = [python, 'tools/package_agent_integrations.py', '--host', 'all']
    modules = os.environ.get('CODECALLNAVIGATOR_TS_NODE_MODULES', 'MISSING_NODE_MODULES')
    path = str(root / 'dist/CodeCallNavigatorEngine/bin') + os.pathsep + os.environ.get('PATH', '')
    return [
        Step('agent-integrations-check', [*package, '--check']),
        Step('agent-integrations-package', [*package, '--output', str(output / 'agent-integrations')]),
        Step('agent-cli-journeys', [python, 'tools/agent_journeys.py', '--node-modules', modules,
                                  '--output', str(output / 'agent-journeys.json')], args.timeout,
             {'PATH': path}),
    ]
