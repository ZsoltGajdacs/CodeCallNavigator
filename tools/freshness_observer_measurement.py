"""Measured ownership of the public CLI observation stream."""
import json
import array
import fcntl
import os
import queue
import select
import socket
import subprocess
import tempfile
import threading
import termios
import time
from dataclasses import dataclass

from freshness_measurement import ProcessMeasurement, inventory_owned, kill_owned, response, run, stop_owned


def request(project):
    return {"version": 2, "project": str(project.root), "roots": project.roots,
            "query": {"kind": "freshness", "observation_version": 1}}


def observe_once(executable, project):
    envelope = response(run([str(executable), "query"], request(project)))
    if envelope.get("error"):
        raise RuntimeError(f"Freshness query failed: {envelope}")
    return envelope["result"]


def checked_event(event):
    if event is None:
        raise RuntimeError("Observer ended before a qualifying observation")
    if isinstance(event, Exception):
        raise RuntimeError(f"Cannot read observer: {event}")
    if event.get("error"):
        raise RuntimeError(f"Observer reported failure: {event}")
    return event


def decode_event(line):
    event = json.loads(line)
    if not isinstance(event, dict):
        raise ValueError("Observer event must be an object")
    return event


@dataclass(frozen=True)
class Boundary:
    sequence: int = -1
    received: float = 0


class BoundedOutput:
    """A real bounded stdout transport for explicit undrained-output fault probes.

    Fill only after the observer reader has stopped. Filler bytes exercise kernel
    backpressure and must never be interpreted as application protocol frames.
    """
    def __init__(self):
        self.reader, self.writer = socket.socketpair(socket.AF_UNIX, socket.SOCK_STREAM)
        self.writer.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)

    def pending_bytes(self):
        pending = array.array('i', [0])
        fcntl.ioctl(self.reader.fileno(), termios.FIONREAD, pending, True)
        return pending[0]

    def writable(self):
        poller = select.poll()
        poller.register(self.writer, select.POLLOUT)
        return any(events & select.POLLOUT for _, events in poller.poll(0))

    def fill(self):
        blocking = self.writer.getblocking()
        self.writer.setblocking(False)
        try:
            self.fill_send_buffer()
        finally:
            self.writer.setblocking(blocking)
        return self.queued_backpressure()

    def fill_send_buffer(self):
        while True:
            try:
                self.writer.send(b'x' * 4096, socket.MSG_DONTWAIT)
            except BlockingIOError:
                return

    def queued_backpressure(self):
        if self.writable():
            raise RuntimeError('Bounded stdout transport did not establish kernel backpressure')
        queued = self.pending_bytes()
        if queued <= 0:
            raise RuntimeError('Bounded stdout transport contains no unread bytes')
        return queued

    def close(self):
        self.writer.close()
        self.reader.close()


@dataclass(frozen=True)
class ObserverOptions:
    on_event: object = None
    output: BoundedOutput | None = None


class Observer:
    """Keep stdin open, timestamp complete output lines, and reap on every exit."""
    def __init__(self, command, initial_request, timeout, options=ObserverOptions()):
        self.timeout, self.events = timeout, queue.Queue()
        self.on_event, self.output = options.on_event, options.output
        self.stop_reader, self.output_eof = threading.Event(), False
        self.stderr = tempfile.TemporaryFile()
        self.measurement = self.start_measurement(command)
        self.reader = threading.Thread(target=self.read)
        self.reader.start()
        try:
            self.measurement.send(initial_request, keep_open=True)
        except BaseException:
            self.close()
            raise

    def start_measurement(self, command):
        stdout = self.output.writer if self.output is not None else subprocess.PIPE
        try:
            measurement = ProcessMeasurement(command, stdout, self.stderr)
            if self.output is not None:
                measurement.process.stdout = self.output.reader
            return measurement
        except BaseException:
            self.stderr.close()
            if self.output is not None:
                self.output.close()
            raise

    def read(self):
        try:
            for line in self.lines():
                received = time.monotonic()
                event = decode_event(line)
                if self.on_event is not None:
                    self.on_event(received, event)
                self.events.put((received, event))
        except (OSError, ValueError) as error:
            self.events.put((time.monotonic(), error))
        finally:
            self.events.put((time.monotonic(), None))

    def blocks(self):
        descriptor = self.measurement.process.stdout.fileno()
        os.set_blocking(descriptor, False)
        while self.readable(descriptor):
            block = os.read(descriptor, 65536)
            if not block:
                self.output_eof = True
                return
            yield block

    def readable(self, descriptor):
        while True:
            timeout = 0 if self.stop_reader.is_set() else .05
            ready, _, _ = select.select([descriptor], [], [], timeout)
            if ready:
                return True
            if self.stop_reader.is_set():
                return False

    def lines(self):
        pending = b""
        for block in self.blocks():
            pending += block
            while b"\n" in pending:
                line, _, pending = pending.partition(b"\n")
                yield line
        if pending:
            raise ValueError("Observer ended with an incomplete output line")

    def wait(self, predicate, boundary=Boundary()):
        deadline = time.monotonic() + self.timeout
        while True:
            received, event = self.next_event(deadline)
            if event.get("sequence", -1) <= boundary.sequence:
                continue
            if received < boundary.received:
                continue
            if event.get("observation") and predicate(event["observation"]):
                return received, event

    def next_event(self, deadline):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError("Timed out waiting for a verified observation")
        try:
            received, event = self.events.get(timeout=remaining)
        except queue.Empty as error:
            raise RuntimeError("Observer produced no qualifying observation") from error
        return received, checked_event(event)

    def close(self):
        process = self.measurement.process
        owned = {}
        if process.poll() is None:
            inventory_owned(process, owned, self.measurement.cleanup_errors)
        try:
            self.measurement.close_input()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.measurement.timed_out = True
                self.measurement.cleanup_errors.extend(stop_owned(process, 5))
        finally:
            kill_owned(owned, self.measurement.cleanup_errors)
            self.measurement.finish()
            if self.output is not None:
                self.output.writer.close()
            self.stop_reader.set()
            self.reader.join(timeout=5)
            process.stdout.close()
            if self.output is not None:
                self.output.close()
        self.stderr.seek(0)
        report = self.measurement.resource_report() | {
            "stderr": self.stderr.read().decode(errors="replace"),
            "reader_stopped": not self.reader.is_alive(), "output_eof": self.output_eof}
        self.stderr.close()
        return report
