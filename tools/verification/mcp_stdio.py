"""Bounded JSON-line MCP transport with process-group cleanup."""
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

class QualityError(RuntimeError):
    """An unavailable or invalid quality check must never become a passing gate."""


class McpClient:
    """One bounded JSON-line session; all child processes are stopped on exit."""

    def __init__(self, command: str, root: Path, timeout: float):
        self.deadline = time.monotonic() + timeout
        self.sequence = 0
        self.buffer = b''
        self.process = subprocess.Popen(
            [command], cwd=root, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, start_new_session=True,
        )
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)

    def __enter__(self):
        try:
            self.request('initialize', {
                'protocolVersion': '2024-11-05', 'capabilities': {},
                'clientInfo': {'name': 'codecallnavigator-verification', 'version': '1'},
            })
            self.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
            return self
        except BaseException:
            self.close()
            raise

    def __exit__(self, *_):
        self.close()

    def close(self):
        self.selector.close()
        try:
            available = self.stop_processes()
        finally:
            for stream in (self.process.stdin, self.process.stdout):
                stream.close()
        if not available:
            raise QualityError('CodeScene process-group cleanup unavailable; descendant termination could not be verified')

    def stop_processes(self):
        available = self._signal(signal.SIGTERM)
        try:
            self.process.wait(timeout=1)
        except subprocess.TimeoutExpired:
            available = self._signal(signal.SIGKILL) and available
            self.process.wait(timeout=1)
        # The server may exit before descendants that ignored SIGTERM.
        return self._signal(signal.SIGKILL) and available

    def _signal(self, sig):
        try:
            os.killpg(self.process.pid, sig)
            return True
        except ProcessLookupError:
            return True
        except PermissionError:
            if self.process.poll() is None:
                self.process.send_signal(sig)
            return False

    def send(self, message):
        try:
            self.process.stdin.write((json.dumps(message) + '\n').encode())
            self.process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise QualityError('CodeScene server closed its input') from error

    def request(self, method, parameters):
        self.sequence += 1
        request_id = self.sequence
        self.send({'jsonrpc': '2.0', 'id': request_id, 'method': method, 'params': parameters})
        while True:
            message = self.read_message()
            if message.get('id') != request_id:
                continue
            if 'error' in message:
                raise QualityError(f'CodeScene rejected {method}')
            if 'result' not in message:
                raise QualityError(f'CodeScene omitted the result for {method}')
            return message['result']

    def read_message(self):
        line = self.read_line()
        try:
            message = json.loads(line)
        except (ValueError, UnicodeError) as error:
            raise QualityError('CodeScene returned invalid JSON-line protocol data') from error
        if not isinstance(message, dict):
            raise QualityError('CodeScene returned a non-object protocol message')
        return message

    def read_line(self):
        while b'\n' not in self.buffer:
            self.read_chunk()
        line, self.buffer = self.buffer.split(b'\n', 1)
        return line

    def read_chunk(self):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0 or not self.selector.select(remaining):
            raise QualityError('CodeScene check timed out')
        chunk = os.read(self.process.stdout.fileno(), 65536)
        if not chunk:
            raise QualityError('CodeScene server terminated before completing its response')
        self.buffer += chunk
        if len(self.buffer) > 16 * 1024 * 1024:
            raise QualityError('CodeScene response exceeded the size limit')

    def tool(self, name, arguments):
        result = self.request('tools/call', {'name': name, 'arguments': arguments})
        if not isinstance(result, dict) or result.get('isError'):
            raise QualityError(f'{name} returned a tool error')
        return result
