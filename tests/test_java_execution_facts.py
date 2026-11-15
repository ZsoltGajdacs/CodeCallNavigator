"""Real javac facts distinguish whole methods, final invocations and deferred regions."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from test_java_context import LIB, installed_jdk, require_analyzer, run, tsv
from test_java_lambda_correlation import EXPORTS

SOURCE = (Path(__file__).resolve().parents[1] / "fixtures/flow/java-execution/Flow.java").read_text()


def objects(value):
    pending = [value]
    while pending:
        current = pending.pop()
        if isinstance(current, dict):
            yield current
            pending.extend(reversed(list(current.values())))
        if isinstance(current, list):
            pending.extend(reversed(current))


def calls(function):
    return [node for node in objects(function["body"]) if "targets" in node]


class JavaExecutionFactsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        home = installed_jdk()
        cls.java, cls.javac = home / "bin/java", home / "bin/javac"
        require_analyzer()

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="java-execution-facts-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def inventory(self, source=SOURCE, dependencies=(), source_after_compile=None):
        file = self.root / "src/fixture/Flow.java"
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(source)
        classes = self.root / "classes"
        classpath = os.pathsep.join(map(str, (classes, *dependencies)))
        run(self.javac, "-g", "-cp", classpath, "-d", classes, *sorted(file.parent.glob("*.java")))
        if source_after_compile is not None:
            file.write_text(source_after_compile)
        output = self.root / "inventory.tsv"
        run(self.java, *EXPORTS, "-cp", LIB / "*", "codecallnavigator.bytecode.SourceInventory",
            output, classpath, classes, self.root / "src")
        return [json.loads(row["start_line"]) for row in tsv(output)
                if next(iter(row.values())) == "flow"]

    def test_explicit_methods_are_synchronous_without_return_type_inference(self):
        flows = self.inventory()
        future = next(flow for flow in flows if ":future()" in flow["symbol"])
        self.assertEqual(future["execution"], "synchronous")
        constructor = next(flow for flow in flows if ":<init>()" in flow["symbol"])
        self.assertNotIn("execution", constructor)
        lambdas = [flow for flow in flows if flow.get("lambda_owner")]
        self.assertTrue(lambdas)
        self.assertTrue(all("execution" not in flow for flow in lambdas))
        self.assertFalse(any(flow["symbol"].startswith("fixture.Data:") for flow in flows))
        self.assertFalse(any(flow["symbol"] == "fixture.Concrete:get()|java.lang.Object" for flow in flows))

    def test_final_invocations_follow_receivers_arguments_and_keep_compiler_dispatch(self):
        flows = self.inventory()
        method = next(flow for flow in flows if ":run(fixture.Contract)" in flow["symbol"])
        sites = calls(method)
        self.assertEqual([site["label"] for site in sites],
            ["receiver()", "arg()", "receiver().hidden(arg())", "fixed()", "contract.hit()", '"x".length()'])
        self.assertEqual([site["invocation"]["dispatch"] for site in sites],
            ["direct", "direct", "direct", "direct", "virtual", "direct"])
        for value in objects(method["body"]):
            if value.get("kind") == "value" and value.get("calls"):
                self.assertEqual(len(value["calls"]), 1)
                self.assertFalse(value["regions"])
                site = value["calls"][0]
                self.assertEqual(site["span"], value["span"])
                self.assertTrue(site["invocation"]["isolated_ordered"])
                self.assertEqual(site["invocation"]["timing"], "synchronous")
        final_owner = next(flow for flow in flows if flow["symbol"] == "fixture.FinalOwner:run()|int")
        self.assertEqual(calls(final_owner)[0]["invocation"]["dispatch"], "direct")

    def test_constructors_and_member_references_never_receive_invocation_proof(self):
        flows = self.inventory()
        method = next(flow for flow in flows if ":refs()" in flow["symbol"])
        special = [site for site in calls(method) if site["label"] in {"new Flow()", "this::fixed"}]
        self.assertEqual(len(special), 2)
        self.assertTrue(all("invocation" not in site for site in special))

    def test_ambiguous_body_recovery_does_not_prove_an_invocation(self):
        source = """package fixture;
public class Flow { static int pick(String x){return 1;} static int pick(Integer x){return 2;}
  int run(){return pick((String)null);} }
"""
        neighbor = self.root / "src/fixture/Neighbor.java"
        neighbor.parent.mkdir(parents=True, exist_ok=True)
        neighbor.write_text("package fixture; class Neighbor { int run(){return 1;} }")
        flows = self.inventory(source, source_after_compile=source.replace("pick((String)null)", "pick(null)"))
        method = next(flow for flow in flows if ":run()" in flow["symbol"])
        site = next(site for site in calls(method) if site["label"] == "pick(null)")
        self.assertNotIn("invocation", site)
        self.assertNotIn("execution", method)
        neighbor = next(flow for flow in flows if flow["symbol"] == "fixture.Neighbor:run()|int")
        self.assertEqual(neighbor["execution"], "synchronous")

    def test_unowned_real_compiler_error_leaves_all_fact_proof_unknown(self):
        fixture = Path(__file__).resolve().parents[1] / "fixtures/flow/java-execution/AttributionProbe.java"
        classes = self.root / "probe"
        run(self.javac, "-cp", LIB / "*", "-d", classes, fixture)
        source = self.root / "Flow.java"
        source.write_text("class Flow { int run(){return 1;} }")
        result = run(self.java, "-cp", os.pathsep.join(map(str, (classes, LIB / "*"))),
                     "codecallnavigator.bytecode.AttributionProbe", source)
        self.assertIn("UNOWNED_COMPILER_ERROR_REMAINS_UNKNOWN", result.stdout)

    def test_reactor_methods_are_synchronous_but_region_values_are_not_isolated(self):
        repository = Path.home() / ".m2/repository"
        dependencies = [repository / "io/projectreactor/reactor-core/3.8.7/reactor-core-3.8.7.jar",
                        repository / "org/reactivestreams/reactive-streams/1.0.4/reactive-streams-1.0.4.jar"]
        self.assertTrue(all(path.is_file() for path in dependencies), "Pinned Reactor fixture dependencies are required")
        flows = self.inventory("""package fixture;
import reactor.core.publisher.Mono;
public class Flow { static int leaf(int x){ return x; }
  Mono<Integer> reactive(){ return Mono.just(1).map(x -> leaf(x)); }
}""", dependencies)
        method = next(flow for flow in flows if ":reactive()" in flow["symbol"])
        self.assertEqual(method["execution"], "synchronous")
        values = [value for value in objects(method["body"]) if value.get("calls") and value.get("regions")]
        self.assertEqual(len(values), 2)
        self.assertTrue(all("invocation" not in call for value in values for call in value["calls"]))
