"""Publication measurement invariants without substituting compiler behavior."""
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import time
from types import SimpleNamespace
import unittest

from freshness_scenarios import Project
from measure_automatic_refresh import PublicationBinding, collect_completion, sample, wait_publication
from measure_freshness_idle import parse_cpu_time, validate_idle_event


def publication(generation, sequence=1, status="match"):
    return {"kind": "change", "sequence": sequence,
            "observation": {"generation": generation, "input_status": status}}


def completed(generation, run=1, sequence=2):
    return {"kind": "activity", "sequence": sequence,
            "activity": {"state": "published", "generation": generation,
                         "runs_started": run, "metrics": {"analysis": 12}}}


class EventStream:
    """A finite delivered-frame boundary; exhaustion means publication timed out."""
    timeout = 1

    def __init__(self, events, publish=None):
        self.events = iter(events)
        self.publish = publish

    def next_event(self, deadline):
        try:
            event = next(self.events)
        except StopIteration as error:
            raise RuntimeError("No expected publication arrived") from error
        if self.publish is not None:
            self.publish()
        if isinstance(event, tuple):
            return event
        return time.monotonic(), event


class AutomaticPublicationTests(unittest.TestCase):
    def test_idle_cpu_time_preserves_centiseconds_and_hour_components(self):
        self.assertAlmostEqual(parse_cpu_time(" 0:00.05 "), .05)
        self.assertAlmostEqual(parse_cpu_time("1:23.45"), 83.45)
        self.assertAlmostEqual(parse_cpu_time("1:02:03.4"), 3723.4)

    def test_failed_zero_run_monitors_cannot_count_as_healthy_idle(self):
        for state in ("unavailable", "failed", "cancelled"):
            with self.subTest(state=state):
                with self.assertRaisesRegex(RuntimeError, "stopped functioning"):
                    validate_idle_event({"activity": {"state": state, "runs_started": 0}}, "saved")
        validate_idle_event({"activity": {"state": "watching", "runs_started": 0}}, "saved")

    def test_unrelated_generation_never_counts_as_expected_publication(self):
        stream = EventStream([publication("old-generation")])
        with self.assertRaisesRegex(RuntimeError, "No expected publication"):
            wait_publication(stream, "new-generation", time.monotonic())

    def test_expected_generation_with_stale_inputs_is_rejected(self):
        stream = EventStream([publication("expected", status="stale")])
        with self.assertRaisesRegex(RuntimeError, "remained stale"):
            wait_publication(stream, "expected", time.monotonic())

    def test_failed_unavailable_or_cancelled_activity_cannot_be_success(self):
        for state in ["failed", "unavailable", "cancelled"]:
            with self.subTest(state=state):
                event = publication("expected")
                event["activity"] = {"state": state}
                with self.assertRaisesRegex(RuntimeError, "did not publish"):
                    wait_publication(EventStream([event]), "expected", time.monotonic())

    def test_delayed_old_sequence_and_pre_save_receipt_cannot_qualify(self):
        started = time.monotonic()
        old_sequence = publication("expected", sequence=8)
        pre_save = publication("expected", sequence=9)
        current = publication("expected", sequence=10)
        stream = EventStream([(started + .01, old_sequence),
                              (started - .01, pre_save), (started + .02, current)])
        received, events = wait_publication(stream, "expected", started, after_sequence=8)
        self.assertEqual(received, started + .02)
        self.assertEqual([item["event"] for item in events], [current])

    def test_future_unrelated_frames_do_not_end_the_measurement(self):
        progress = {"kind": "future_progress", "sequence": 11,
                    "activity": {"state": "analyzing"}}
        unrelated = publication("other", sequence=12)
        current = publication("expected", sequence=13, status="unverifiable")
        _, events = wait_publication(EventStream([progress, unrelated, current]),
                                     "expected", time.monotonic(), after_sequence=10)
        self.assertEqual([item["event"] for item in events], [progress, unrelated, current])

    def test_compiler_metrics_are_collected_after_timed_generation_receipt(self):
        before = completed("older", sequence=3)
        after = completed("expected", sequence=4)
        events = [{"received_after_save_ms": 12, "event": publication("expected")}]
        result = collect_completion(EventStream([before, after]), "expected", time.monotonic(), events)
        self.assertEqual(result, after["activity"])
        self.assertEqual(events[-1]["event"], after)
        self.assertEqual(result["metrics"], {"analysis": 12})

    def test_already_delivered_compiler_metrics_do_not_wait_for_duplicate_activity(self):
        event = completed("expected")
        events = [{"received_after_save_ms": 10, "event": event}]
        result = collect_completion(EventStream([]), "expected", time.monotonic(), events)
        self.assertEqual(result, event["activity"])

    def project(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        project = Project(Path(temporary.name), "maven", None,
                          Path(__file__).resolve().parents[1])
        project.reset()
        artifact = project.root / ".codecallnavigator/graph.json"
        artifact.parent.mkdir()
        return SimpleNamespace(project=project), artifact

    def test_full_artifact_mismatch_is_retained_and_raises(self):
        session, artifact = self.project()
        expected = b'{"graph":{"nodes":[],"metadata":"expected"}}'
        actual = b'{"graph":{"nodes":[],"metadata":"different"}}'
        artifact.write_bytes(actual)
        stream = EventStream([publication("expected-generation"), completed("expected-generation")])
        with self.assertRaisesRegex(RuntimeError, "differs from independent analysis") as failure:
            binding = PublicationBinding({1: expected}, {1: "expected-generation"})
            sample(session, stream, binding, 0)
        retained = Path(str(failure.exception).split(": ", 1)[1])
        self.addCleanup(shutil.rmtree, retained)
        self.assertEqual((retained / "actual.json").read_bytes(), actual)
        self.assertEqual((retained / "expected.json").read_bytes(), expected)

    def test_an_extra_automatic_analysis_cannot_be_counted_as_one_publication(self):
        session, artifact = self.project()
        artifact.write_bytes(b"reference")
        binding = PublicationBinding({1: b"reference"}, {1: "expected"})
        stream = EventStream([publication("expected"), completed("expected", run=2)])
        with self.assertRaisesRegex(RuntimeError, "Unexpected automatic analysis count"):
            sample(session, stream, binding, 0)

    def test_saved_variants_alternate_and_first_is_separate_from_warm(self):
        session, artifact = self.project()
        references = {variant: json.dumps({"graph": {"body": variant}}).encode()
                      for variant in [0, 1]}
        generations = {0: "a" * 64, 1: "b" * 64}
        binding = PublicationBinding(references, generations)
        records = []
        for iteration, variant in enumerate([1, 0, 1]):
            def publish():
                self.assertEqual(session.project.source.read_text(),
                                 session.project.text(variant + 1))
                artifact.write_bytes(references[variant])
            event = publication(generations[variant], sequence=iteration * 2 + 1)
            terminal = completed(generations[variant], iteration + 1, iteration * 2 + 2)
            record = sample(session, EventStream([event, terminal], publish), binding, iteration)
            records.append(record)
            self.assertTrue(record["equivalent"])
            self.assertEqual(record["reference_sha256"],
                             hashlib.sha256(references[variant]).hexdigest())
        self.assertEqual([record["variant"] for record in records], [1, 0, 1])
        self.assertEqual([record["phase"] for record in records], ["first", "warm", "warm"])


if __name__ == "__main__":
    unittest.main()
