"""Resolve the installed Graphviz recorded in the analyzer lock for verification."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess


def locked_version(root):
    return json.loads((root / 'analyzers.lock.json').read_text())['graphviz']['version']


def supports_version(output, expected):
    match = re.search(r'graphviz version (\d+\.\d+\.\d+)(?:\s|$)', output)
    return match is not None and match.group(1) == expected


def probe(command, *, include_stderr=True):
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=True, timeout=5)
    except (OSError, subprocess.SubprocessError):
        return ''
    return result.stdout + result.stderr if include_stderr else result.stdout


def locked_bin(root):
    """Keep a matching PATH; otherwise use a matching installed Homebrew keg."""
    expected = locked_version(root)
    dot = shutil.which('dot')
    if dot and supports_version(probe([dot, '-V']), expected):
        return None
    brew = shutil.which('brew')
    if brew is None:
        return None
    cellar = probe([brew, '--cellar', 'graphviz'], include_stderr=False).strip()
    if not Path(cellar).is_absolute():
        return None
    directory = Path(cellar) / expected / 'bin'
    if supports_version(probe([str(directory / 'dot'), '-V']), expected):
        return directory
    return None


def prepend_locked_bin(directory, steps):
    default_path = os.environ.get('PATH', os.defpath)
    for step in steps:
        stage_path = step.environment.get('PATH', default_path)
        step.environment['PATH'] = os.pathsep.join([str(directory), stage_path])
