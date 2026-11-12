#!/usr/bin/env python3
"""Verify an installed CLI with real compiler queries after removing its download."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from agent_cli import AcceptanceError, Client, require
from agent_journeys import fixture, protocol


def run(package, modules, output):
    package, modules, output = package.resolve(), modules.resolve(), output.resolve()
    with tempfile.TemporaryDirectory(prefix='installed-cli-') as directory:
        work = Path(directory).resolve()
        download = work / 'extracted download'
        shutil.copytree(package, download, symlinks=True)
        prefix = work / 'user prefix'
        environment = dict(os.environ)
        for key in ['CODECALLNAVIGATOR_RESOURCES', 'CODECALLNAVIGATOR_ENGINE',
                    'CODECALLNAVIGATOR_TEST_ENGINE']:
            environment.pop(key, None)
        subprocess.run([sys.executable, str(download / 'install.py'), '--prefix', str(prefix)],
                       cwd=work, env=environment, check=True, timeout=120)
        shutil.rmtree(download)
        environment['PATH'] = str(prefix / 'bin') + os.pathsep + environment.get('PATH', '')
        environment['CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT'] = str(modules / 'typescript')
        require(shutil.which('ccnavigator', path=environment['PATH']) == str(prefix / 'bin/ccnavigator'),
                'Installed command must resolve through PATH')
        project = work / 'project'
        project.mkdir()
        fixture(project, modules)
        previous = Path.cwd()
        try:
            os.chdir(work)
            client = Client('ccnavigator', project, environment)
            help_text = client.command('installed-help', ['--help'])
            require('ccnavigator query' in help_text, 'Installed help must name ccnavigator')
            freshness = protocol(client, project)
        finally:
            os.chdir(previous)
        report = {'status': 'passed', 'download_removed': True,
                  'initial_freshness': freshness, 'records': client.records}
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2).replace(str(work), '<installation-test>') + '\n')
        print(f'Passed {len(client.records)} installed CLI operations; report: {output}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', type=Path, required=True)
    parser.add_argument('--node-modules', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.package, args.node_modules, args.output)


if __name__ == '__main__':
    try:
        main()
    except (AcceptanceError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        raise SystemExit(f'Installed CLI acceptance failed: {error}') from error
