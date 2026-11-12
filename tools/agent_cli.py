"""Bounded standalone CLI compatibility client; plugins use native MCP tools."""
import io
import json
import subprocess
import tempfile
import time

from verification.runner import stop_group


class AcceptanceError(RuntimeError):
    """A missing prerequisite or failed observable acceptance assertion."""


def require(condition, message):
    if not condition:
        raise AcceptanceError(message)


def execute(command, environment, request=None):
    """Run a bounded owned CLI, preserving its JSON and structured error exit."""
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=output,
                                   stderr=errors, env=environment, start_new_session=True)
        try:
            process.communicate(request, timeout=60)
        except (subprocess.TimeoutExpired, KeyboardInterrupt):
            log = io.StringIO()
            require(stop_group(process, log), 'CLI descendant cleanup unavailable')
            raise
        output.seek(0)
        data = output.read(32 * 1024 * 1024 + 1)
        require(len(data) <= 32 * 1024 * 1024, 'CLI response exceeded 32 MiB')
        errors.seek(0)
        return process.returncode, data, errors.read(65536).decode(errors='replace')


class Client:
    """The CLI owns graph semantics; this client records process observations."""

    def __init__(self, engine, root, environment):
        self.engine = str(engine)
        self.root = root
        self.environment = environment
        self.records = []

    def command(self, scenario, arguments):
        started = time.monotonic()
        code, data, stderr = execute([self.engine, *arguments], self.environment)
        self.records.append({'scenario': scenario, 'command': arguments,
                             'milliseconds': round((time.monotonic() - started) * 1000, 2),
                             'exit_code': code, 'stdout': data.decode(), 'stderr': stderr})
        require(code == 0, f'{scenario}: CLI failed with {code}: {stderr}')
        return data.decode()

    def query(self, scenario, query, generation=None, error=None):
        request = {'version': 1, 'project': str(self.root), 'query': query}
        if generation is not None:
            request['expected_generation'] = generation
        started = time.monotonic()
        code, data, stderr = execute([self.engine, 'query'], self.environment,
                                    json.dumps(request).encode())
        envelope = json.loads(data)
        self.records.append({'scenario': scenario, 'request': request,
                             'milliseconds': round((time.monotonic() - started) * 1000, 2),
                             'exit_code': code, 'bytes': len(data),
                             'response': envelope, 'stderr': stderr})
        actual = (envelope.get('error') or {}).get('code')
        require(actual == error, f'{scenario}: expected error {error}, got {actual}')
        require(code == (2 if error else 0), f'{scenario}: incorrect CLI exit {code}')
        require(envelope.get('version') == 1, f'{scenario}: unsupported response version')
        return envelope

    def analyze(self, scenario):
        return self.command(scenario, ['analyze', '--project', str(self.root),
                                       str(self.root / 'tsconfig.json')])
