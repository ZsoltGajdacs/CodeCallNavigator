"""Acceptance must depend on real native presentations of changed camera positions."""
import json
from pathlib import Path
import tempfile
import unittest

import pan_profile
import pan_profile_report
import hashlib


class NativeAcceptance(unittest.TestCase):
    def test_binary_hash_identifies_exact_bytes_on_python39(self):
        with tempfile.TemporaryDirectory() as temporary:
            binary = Path(temporary) / "codegraph"
            binary.write_bytes(b"native-release-binary")
            self.assertEqual(pan_profile.binary_hash(binary), hashlib.sha256(binary.read_bytes()).hexdigest())

    def trace(self, directory, **options):
        fps = options.get("fps", 60)
        clock = options.get("clock", "wall")
        moving = options.get("moving", True)
        presents = options.get("presents", True)
        rows = [{"type": "header", "clock": clock, "scale": 2, "width": 1900, "height": 1100}]
        names = ["sample", "drag", "drag_seconds", "camera.x", "camera.y", "camera.zoom",
                 "backing_scale", "viewport.width", "viewport.height", "target.width", "target.height"]
        descriptions = [{"id": i + 1, "name": "codegraph." + name, "kind": "level"} for i, name in enumerate(names)]
        for drag in range(3):
            for step in range(fps * 10 + 1):
                row = {"t": (drag * 11 + step / fps) * 1000,
                       "c": [[1, len(rows)], [2, drag], [3, step / fps],
                             [4, step if moving else 0], [5, 0], [6, 1], [7, 2],
                             [8, 1900], [9, 1100], [10, 3800], [11, 2200]]}
                if len(rows) == 1:
                    row["cm"] = descriptions
                if presents:
                    row["present_ms"] = row["t"] + 8
                rows.append(row)
        (directory / "native.jsonl").write_text("\n".join(json.dumps(row) for row in rows))

    def evaluate(self, **kwargs):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.trace(directory, **kwargs)
            return pan_profile.summarize(directory)

    def evaluate_modified(self, mutate, expected_scale=None):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.trace(directory)
            rows = pan_profile.read_rows(directory / "native.jsonl")
            mutate(rows)
            (directory / "native.jsonl").write_text("\n".join(json.dumps(row) for row in rows))
            return pan_profile.summarize(directory, expected_scale)

    def test_native_changed_camera_at_sixty_fps_passes(self):
        result = self.evaluate()
        self.assertEqual(result["status"], "passed")
        self.assertTrue(all(drag["full_drag_covered"] for drag in result["drags"]))

    def test_virtual_clock_never_passes_even_with_present_fields(self):
        self.assertEqual(self.evaluate(clock="virtual")["status"], "invalid")

    def test_no_presentation_timestamps_are_unavailable(self):
        self.assertEqual(self.evaluate(presents=False)["status"], "unavailable")

    def test_repeated_idle_camera_cannot_count_as_fps(self):
        self.assertEqual(self.evaluate(moving=False)["status"], "failed")

    def test_twenty_fps_fails(self):
        self.assertEqual(self.evaluate(fps=20)["status"], "failed")

    def test_multiple_camera_rows_with_the_same_present_time_do_not_pass(self):
        def repeated_time(rows):
            for row in rows[1:]:
                row["present_ms"] = 10
        self.assertEqual(self.evaluate_modified(repeated_time)["status"], "failed")

    def test_rare_long_presentation_stall_extends_the_measured_duration(self):
        def stall(rows):
            for row in rows[101:]:
                row["present_ms"] += 20_000
        result = self.evaluate_modified(stall)
        self.assertFalse(result["drags"][0]["passed"])
        self.assertLess(result["drags"][0]["fps"], 30)

    def test_native_scale_must_match_the_requested_monitor(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            self.trace(directory)
            self.assertEqual(pan_profile.summarize(directory, 1)["status"], "invalid")

    def test_live_surface_scale_and_size_override_the_startup_header(self):
        result = self.evaluate_modified(lambda rows: rows[0].update(scale=1, width=800, height=600), 2)
        self.assertEqual(result["status"], "passed")
        self.assertEqual(result["native_size_points"], [1900, 1100])

    def test_zero_gpu_durations_are_never_reported_as_measured_zero_cost(self):
        frames = [({}, {"gpu_sample": 1, "gpu_ms": 0}), ({}, {"gpu_sample": 2, "gpu_ms": 2.5})]
        self.assertEqual(pan_profile_report.gpu_samples(frames), {2: 2.5})

    def test_long_missing_presentation_gap_is_not_hidden_by_cpu_fps(self):
        def dropped_frames(rows):
            for index, row in enumerate(rows):
                if index % 3:
                    row.pop("present_ms", None)
        # CPU still submitted 60fps, but only 20fps reached the display.
        self.assertEqual(self.evaluate_modified(dropped_frames)["status"], "failed")


if __name__ == "__main__":
    unittest.main()
