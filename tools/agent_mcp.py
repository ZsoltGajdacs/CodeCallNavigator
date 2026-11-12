"""Bounded native-MCP acceptance transport; the installed server owns CLI access."""
import io
import json
import os
import selectors
import subprocess
import tempfile
import time

from agent_cli import AcceptanceError, require
from verification.runner import stop_group


class Client:
    """Own one real SDK stdio server and record its native tool envelopes."""

    def __init__(self, command, environment):
        self.sequence = 0
        self.buffer = b''
        self.records = []
        self.errors = tempfile.TemporaryFile()
        try:
            self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                            stderr=self.errors, env=environment, start_new_session=True)
        except OSError:
            self.errors.close()
            raise
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    def __enter__(self):
        try:
            self.request('initialize', {
                'protocolVersion': '2025-11-25', 'capabilities': {},
                'clientInfo': {'name': 'native-agent-journeys', 'version': '1'},
            })
            self.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
            return self
        except BaseException:
            self.close()
            raise

    def __exit__(self, error_type, *_):
        code = self.close()
        if error_type is None:
            require(code == 0, f'MCP shutdown failed with exit {code}')

    def close(self):
        try:
            self.process.stdin.close()
        except BrokenPipeError:
            pass
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            require(stop_group(self.process, io.StringIO()), 'MCP descendant cleanup unavailable')
        finally:
            self.selector.close()
            self.process.stdout.close()
            self.errors.close()
        return self.process.returncode

    def send(self, value):
        self.process.stdin.write((json.dumps(value) + '\n').encode())
        self.process.stdin.flush()

    def read(self, deadline):
        while b'\n' not in self.buffer:
            remaining = deadline - time.monotonic()
            require(remaining > 0 and self.selector.select(remaining), 'MCP response deadline expired')
            chunk = os.read(self.process.stdout.fileno(), 65536)
            require(chunk, 'MCP exited before returning its response')
            self.buffer += chunk
            require(len(self.buffer) <= 32 * 1024 * 1024, 'MCP response exceeded 32 MiB')
        line, self.buffer = self.buffer.split(b'\n', 1)
        value = json.loads(line)
        require(isinstance(value, dict), 'MCP response must be a JSON object')
        return value

    def request(self, method, parameters):
        self.sequence += 1
        self.send({'jsonrpc': '2.0', 'id': self.sequence, 'method': method, 'params': parameters})
        deadline = time.monotonic() + 120
        while True:
            response = self.read(deadline)
            if response.get('id') == self.sequence:
                require('error' not in response, f'MCP rejected {method}: {response}')
                return response['result']

    def call(self, scenario, name, arguments, error=None):
        started = time.monotonic()
        result = self.request('tools/call', {'name': name, 'arguments': arguments})
        self.records.append({'scenario': scenario, 'tool': name, 'arguments': arguments,
                             'milliseconds': round((time.monotonic() - started) * 1000, 2),
                             'bytes': len(json.dumps(result).encode()), 'response': result})
        envelope = result.get('structuredContent', {})
        actual = (envelope.get('error') or {}).get('code')
        require(actual == error, f'{scenario}: expected error {error}, got {actual}')
        require(bool(result.get('isError')) == bool(error), f'{scenario}: incorrect isError')
        if name != 'discover_builds':
            require(envelope.get('version') == 1, f'{scenario}: unsupported CLI envelope version')
        return envelope
