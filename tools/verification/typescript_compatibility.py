"""TypeScript compiler eligibility shared with the shipped adapter policy."""
import json
from pathlib import Path
import re
import subprocess


def supports_version(version, supported_minors):
    if not isinstance(version, str):
        return False
    match = re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', version, flags=re.ASCII)
    return match is not None and f'{match[1]}.{match[2]}' in supported_minors


def supported_versions(supported_minors):
    return ' or '.join(f'{minor}.x' for minor in supported_minors)


def loaded_version(package):
    probe = "process.stdout.write(String(require(process.argv[1]).version))"
    result = subprocess.run(['node', '-e', probe, str(package)], capture_output=True,
                            text=True, check=True, timeout=10)
    return result.stdout


def compiler_problems(modules):
    policy_file = Path(__file__).resolve().parents[1] / 'typescript/compatibility.json'
    supported_minors = json.loads(policy_file.read_text())['supported_minors']
    package = modules / 'typescript'
    version = json.loads((package / 'package.json').read_text())['version']
    if not supports_version(version, supported_minors):
        return [f'typescript: expected {supported_versions(supported_minors)}, found {version}']
    try:
        actual = loaded_version(package)
    except (OSError, subprocess.SubprocessError):
        return [f'typescript: Node could not load the compiler at {package}']
    if actual != version:
        return [f'typescript: compiler at {package} reports {actual}; package.json declares {version}']
    return []
