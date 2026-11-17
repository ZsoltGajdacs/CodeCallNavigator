"""TypeScript compiler eligibility shared with the shipped adapter policy."""
import json
from pathlib import Path
import re


def supports_version(version, supported_minors):
    if not isinstance(version, str):
        return False
    match = re.fullmatch(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)', version, flags=re.ASCII)
    return match is not None and f'{match[1]}.{match[2]}' in supported_minors


def supported_versions(supported_minors):
    return ' or '.join(f'{minor}.x' for minor in supported_minors)


def compiler_problems(modules):
    policy_file = Path(__file__).resolve().parents[1] / 'typescript/compatibility.json'
    supported_minors = json.loads(policy_file.read_text())['supported_minors']
    version = json.loads((modules / 'typescript/package.json').read_text())['version']
    if not supports_version(version, supported_minors):
        return [f'typescript: expected {supported_versions(supported_minors)}, found {version}']
    return []
