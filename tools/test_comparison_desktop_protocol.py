"""External native actions must stay inside their exact fixture, binding and operation sequence."""
import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
from unittest.mock import patch
import unittest

from comparison_desktop_protocol import ORDER, read_request, validate
from comparison_desktop_facts import expected_side, remove_legacy_source


class ComparisonProtocolTests(unittest.TestCase):
    def setUp(self):
        self.fixture = SimpleNamespace(project=Path("/private/tmp/owned-project"),
                                       config={"initial": {"comparison_id": "a" * 64}})
        self.request = {"version": 2, "sequence": 1, "operation": "capture", "checkpoint": "missing-source",
                        "project": str(self.fixture.project), "window_id": 123, "comparison_id": "a" * 64}

    def test_ordinary_version_one_capture_keeps_its_existing_request_and_output_path(self):
        from csharp_desktop_acceptance import capture_window
        fixture = SimpleNamespace(entry="ordinary/root", generation="generation")
        request = {"version": 1, "symbol": fixture.entry, "generation": fixture.generation,
                   "viewport": [1050, 680], "window_id": 123}
        with patch("csharp_desktop_acceptance.capture_image", return_value={"window_id": 123}) as capture:
            self.assertEqual(capture_window(request, Path("/owned/output"), fixture), {"window_id": 123})
            capture.assert_called_once_with(123, Path("/owned/output/native.png"))

    def test_exact_capture_envelope_and_every_fixed_checkpoint_are_required(self):
        validate(self.request, 1, self.fixture, 123)
        for key, value in (("version", 1), ("sequence", 2), ("operation", "move_after_ref"),
                           ("checkpoint", "manual-pair"), ("project", "/another/project"),
                           ("window_id", 124), ("comparison_id", "b" * 64), ("unknown", True)):
            invalid = dict(self.request, **{key: value})
            with self.assertRaises(ValueError, msg=key):
                validate(invalid, 1, self.fixture, 123)
        self.assertEqual(len(ORDER), 7)

    def test_non_capture_may_not_carry_window_or_binding_metadata(self):
        request = {"version": 2, "sequence": 4, "operation": "move_after_ref", "checkpoint": "after-ref-moved",
                   "project": str(self.fixture.project)}
        validate(request, 4, self.fixture, 123)
        request["window_id"] = 123
        with self.assertRaises(ValueError):
            validate(request, 4, self.fixture, 123)

    def test_request_reader_rejects_over_budget_input_before_decoding(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "request.json"
            path.write_bytes(b" " * 8193)
            with self.assertRaises(ValueError):
                read_request(path)
            path.write_text("[]")
            with self.assertRaises(ValueError):
                read_request(path)

    def test_only_one_legacy_source_row_is_removed_and_nodes_ir_stay_unchanged(self):
        owner = {"build": "build", "project": "project", "file": "@repo/Legacy.cs"}
        legacy = {**owner, "content": "legacy"}
        other = {**owner, "file": "@repo/Unchanged.cs", "content": "unchanged"}
        artifact = {"graph": {"nodes": [{**owner, "label": "Legacy"}],
                              "flow_inventory": {"functions": ["original"]},
                              "architecture": {"files": [legacy, other]}}}
        original = copy.deepcopy(artifact)
        result = remove_legacy_source(artifact)
        self.assertEqual(artifact, original)
        self.assertEqual(result["graph"]["architecture"]["files"], [other])
        duplicate = copy.deepcopy(artifact)
        duplicate["graph"]["architecture"]["files"].append(legacy)
        with self.assertRaises(ValueError):
            remove_legacy_source(duplicate)

    def test_ready_source_must_match_exact_bom_unicode_crlf_compiler_bytes(self):
        raw = b"\xef\xbb\xbf" + "// café 🧭\r\n".encode()
        side = {"status": "ready", "source": {"origin": "analyzed", "file": "@repo/Changed.cs", "content": raw.decode()},
                "node": {"symbol": "id", "owner": {"language": "csharp", "file": "@repo/Changed.cs"}}, "chart": {}}
        expected_side(side, {"Changed.cs": raw})
        side["source"]["content"] = side["source"]["content"].replace("\r\n", "\n")
        with self.assertRaises(ValueError):
            expected_side(side, {"Changed.cs": raw})


if __name__ == "__main__":
    unittest.main()
