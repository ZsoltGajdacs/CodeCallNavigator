"""Real subprocess checks for measurement stream ordering and bounded cleanup."""
import os
import subprocess
import sys
import time
import unittest

from freshness_observer_measurement import Boundary, BoundedOutput, Observer, ObserverOptions
from freshness_measurement import kill_owned, process_inventory


PRELUDE = 'import json,os,subprocess,sys,time\nsys.stdin.readline()\n'
EMIT = 'print(json.dumps({"sequence":0,"observation":{"input_revision":"ready"}}),flush=True)\n'


class StreamOwnershipTests(unittest.TestCase):
    def observer(self, body, timeout=2, options=ObserverOptions()):
        observer = Observer([sys.executable, "-c", PRELUDE + body], {}, timeout,
                            options)
        self.addCleanup(self.close_remaining, observer)
        return observer

    @staticmethod
    def close_remaining(observer):
        if not observer.stderr.closed:
            observer.close()

    def test_closed_stdin_still_reaps_and_stops_reader(self):
        for output in (None, BoundedOutput()):
            with self.subTest(bounded=output is not None):
                observer = self.observer('os.close(0)\n' + EMIT,
                                         options=ObserverOptions(output=output))
                observer.wait(lambda item: item["input_revision"] == "ready")
                observer.measurement.process.stdin.write(b"unflushed late input")
                report = observer.close()
                self.assertEqual(report["exit_code"], 0)
                self.assertTrue(report["reader_stopped"])
                self.assertTrue(report["output_eof"])

    def test_reused_revision_requires_later_sequence(self):
        body = 'for seq in range(4):\n print(json.dumps({"sequence":seq,"observation":{"input_revision":"same"}}),flush=True)\nsys.stdin.read()\n'
        observer = self.observer(body)
        _, event = observer.wait(lambda item: item["input_revision"] == "same", Boundary(2, 0))
        self.assertEqual(event["sequence"], 3)

    def test_trace_retains_events_filtered_by_the_measurement(self):
        recorded = []
        body = 'for seq in (1,2):\n print(json.dumps({"sequence":seq,"observation":{"input_revision":str(seq)}}),flush=True)\nsys.stdin.read()\n'
        observer = self.observer(body, options=ObserverOptions(
            on_event=lambda received, event: recorded.append((received, event))))
        observer.wait(lambda item: item["input_revision"] == "2")
        self.assertEqual([event["sequence"] for _, event in recorded], [1, 2])
        self.assertTrue(all(received > 0 for received, _ in recorded))

    def test_receipt_before_save_cannot_qualify(self):
        observer = self.observer(EMIT + 'sys.stdin.read()\n', timeout=.1)
        with self.assertRaises(RuntimeError):
            observer.wait(lambda item: True, Boundary(-1, time.monotonic() + 60))

    def test_inherited_stdout_cannot_block_reader_shutdown(self):
        try:
            subprocess.run(["/bin/ps", "-p", str(os.getpid())], check=True, capture_output=True)
        except (OSError, subprocess.SubprocessError):
            self.skipTest("process inventory unavailable")
        body = 'child=subprocess.Popen([sys.executable,"-c","import time;time.sleep(60)"])\nprint(json.dumps({"sequence":0,"observation":{"child":child.pid}}),flush=True)\nsys.stdin.read()\n'
        observer = self.observer(body)
        _, event = observer.wait(lambda item: "child" in item)
        child = event["observation"]["child"]
        identity = {pid: started for pid, _, started in process_inventory() if pid == child}
        self.addCleanup(kill_owned, identity, [])
        started = time.monotonic()
        report = observer.close()
        self.assertLess(time.monotonic() - started, 3)
        self.assertEqual(report["exit_code"], 0)
        self.assertTrue(report["reader_stopped"])
        self.assertTrue(report["output_eof"])
        status = subprocess.run(["/bin/ps", "-p", str(child), "-o", "stat="], capture_output=True, text=True).stdout.strip()
        self.assertTrue(not status or status.startswith("Z"), status)

    def test_bounded_output_proves_real_backpressure_and_releases_both_descriptors(self):
        output = BoundedOutput()
        descriptors = [output.reader.fileno(), output.writer.fileno()]
        try:
            self.assertTrue(output.writable())
            queued = output.fill()
            self.assertTrue(os.get_blocking(output.writer.fileno()))
            self.assertGreater(queued, 0)
            self.assertFalse(output.writable())
            self.assertEqual(output.pending_bytes(), queued)
            self.assertEqual(len(output.reader.recv(queued)), queued)
            self.assertEqual(output.pending_bytes(), 0)
            self.assertTrue(output.writable())
        finally:
            output.close()
        for descriptor in descriptors:
            with self.assertRaises(OSError):
                os.fstat(descriptor)

if __name__ == "__main__":
    unittest.main()
