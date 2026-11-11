"""Exercise protocol boundaries and normalization, without simulating Java analysis."""

import io
from pathlib import Path
import queue
import sys
import tempfile
import time
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import jdt_extract as jdt


def reported_symbol(name="run()", kind=6, line=3):
    location = {"start": {"line": line, "character": 4},
                "end": {"line": line, "character": 7}}
    return {"name": name, "kind": kind, "range": location, "selectionRange": location}


class ProtocolResponses:
    """Supply predefined wire responses to test the collector's boundary handling."""

    def __init__(self, responses):
        self.responses = responses
        self.requests = []
        self.messages = queue.Queue()

    def start(self):
        pass

    def request(self, method, params, **kwargs):
        self.requests.append((method, params))
        return self.responses[method]


class FramingTests(unittest.TestCase):
    def test_utf8_frame_length_counts_bytes(self):
        payload = '{"name":"méthode"}'.encode("utf-8")
        frame = f"Content-Length: {len(payload)}\r\n\r\n".encode("ascii") + payload
        self.assertEqual(jdt.read_message(io.BytesIO(frame)), {"name": "méthode"})

    def test_truncated_payload_raises_instead_of_returning_partial_json(self):
        with self.assertRaisesRegex(jdt.RpcError, "inside a message"):
            jdt.read_message(io.BytesIO(b'Content-Length: 20\r\n\r\n{}'))

    def test_oversized_payload_is_rejected_before_reading(self):
        with self.assertRaisesRegex(jdt.RpcError, "Invalid JDT message size"):
            jdt.read_payload(io.BytesIO(), 256 * 1024 * 1024 + 1)


class SymbolMappingTests(unittest.TestCase):
    def test_nested_parent_identity_and_overloads_are_preserved(self):
        outer = reported_symbol("Example", kind=5, line=0)
        outer["children"] = [reported_symbol("run(int)", line=3),
                             reported_symbol("run(String)", line=5)]
        root = Path("/private/tmp/CodeCallNavigator symbols")
        uri = (root / "Example.java").as_uri()
        declarations = list(jdt.collect_symbols([outer], uri, root))
        self.assertEqual(len(declarations), 3)
        self.assertEqual(declarations[1]["parent"], declarations[0]["id"])
        self.assertNotEqual(declarations[1]["id"], declarations[2]["id"])
        self.assertEqual(declarations[2]["name"], "run(String)")
        self.assertEqual(declarations[2]["file"], "Example.java")

    def test_flat_or_incomplete_symbol_responses_are_rejected(self):
        with self.assertRaisesRegex(jdt.RpcError, "flat/invalid"):
            list(jdt.collect_symbols([{"name": "x", "kind": 6}], "file:///x.java", Path("/")))

    def test_uri_ownership_uses_path_boundaries(self):
        root = Path("/private/tmp/CodeCallNavigator symbols")
        self.assertIsNone(jdt.project_path("jdt://contents/library.class", root))
        self.assertIsNone(jdt.project_path((root.parent / "CodeCallNavigator symbols-other/X.java").as_uri(), root))
        self.assertEqual(jdt.project_path((root / "X.java").as_uri(), root), "X.java")


class CollectionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.source = self.root / "src/main/java"
        self.source.mkdir(parents=True)
        self.java_file = self.source / "Example.java"
        self.java_file.touch()
        self.uri = self.java_file.as_uri()
        self.symbol = reported_symbol()
        self.item = {**self.symbol, "uri": self.uri}
        self.evidence = jdt.new_evidence(self.root)

    def collector(self, **overrides):
        responses = {"java/buildWorkspace": "SUCCEED",
                     "workspace/executeCommand": {"status": True, "data": [
                         {"path": str(self.source), "projectName": "sample"}]},
                     "textDocument/documentSymbol": [self.symbol],
                     "textDocument/prepareCallHierarchy": [self.item],
                     "callHierarchy/outgoingCalls": []}
        responses.update(overrides)
        return jdt.ProjectCollector(ProtocolResponses(responses), self.root, self.evidence)

    def finalize(self):
        jdt.finish_evidence(self.evidence, time.monotonic())
        return self.evidence["complete"]

    def test_failed_build_stops_before_source_or_call_collection(self):
        collector = self.collector(**{"java/buildWorkspace": "WITH_ERROR"})
        with self.assertRaisesRegex(jdt.RpcError, "workspace build did not succeed"):
            collector.run()
        self.assertEqual([method for method, _ in collector.client.requests], ["java/buildWorkspace"])
        self.assertFalse(self.finalize())

    def test_disconnected_method_survives_empty_outgoing_list(self):
        self.collector().run()
        self.assertEqual(len(self.evidence["declarations"]), 1)
        self.assertEqual(self.evidence["declarations"][0]["project"], "sample")
        self.assertEqual(self.evidence["edges"], [])
        self.assertTrue(self.finalize())

    def test_only_analyzer_owned_source_roots_are_scanned(self):
        unowned = self.root / "unowned"
        unowned.mkdir()
        (unowned / "NotInBuild.java").touch()
        self.collector().run()
        self.assertEqual(self.evidence["filesDiscovered"], 1)
        self.assertEqual(self.evidence["declarations"][0]["file"], "src/main/java/Example.java")

    def test_external_targets_are_counted_but_not_connected(self):
        external = {**self.item, "uri": "jdt://contents/java.base/External.class"}
        responses = {"callHierarchy/outgoingCalls": [{"to": self.item, "fromRanges": []},
                                                    {"to": external, "fromRanges": []}]}
        self.collector(**responses).run()
        self.assertEqual(self.evidence["externalRelationshipsExcluded"], 1)
        self.assertEqual(len(self.evidence["edges"]), 1)
        edge = self.evidence["edges"][0]
        self.assertEqual(edge["from"], edge["to"])
        self.assertEqual(edge["certainty"], "analyzer-reported")

    def test_null_hierarchy_is_incomplete(self):
        self.collector(**{"textDocument/prepareCallHierarchy": None}).run()
        self.assertEqual(self.evidence["failures"][0]["stage"], "prepare-call-hierarchy")
        self.assertFalse(self.finalize())

    def test_null_outgoing_calls_are_not_treated_as_no_calls(self):
        self.collector(**{"callHierarchy/outgoingCalls": None}).run()
        self.assertEqual(self.evidence["failures"][0]["stage"], "outgoing-calls")
        self.assertFalse(self.finalize())

    def test_null_document_symbols_are_recorded(self):
        collector = self.collector(**{"textDocument/documentSymbol": None})
        with self.assertRaisesRegex(jdt.RpcError, "No callable"):
            collector.run()
        self.assertEqual(self.evidence["failures"][0]["stage"], "document-symbol")
        self.assertFalse(self.finalize())

    def test_error_diagnostics_prevent_complete_result(self):
        self.collector().run()
        self.evidence["diagnostics"] = {self.uri: [{"severity": 1, "message": "Unresolved type"}]}
        self.assertFalse(self.finalize())
        self.assertEqual(self.evidence["errorDiagnosticCount"], 1)

    def test_server_errors_prevent_complete_result(self):
        self.collector().run()
        self.evidence["serverStatus"] = [{"type": "server-error", "message": "Import failed"}]
        self.assertFalse(self.finalize())
        self.assertEqual(self.evidence["serverErrorCount"], 1)


if __name__ == "__main__":
    unittest.main()
