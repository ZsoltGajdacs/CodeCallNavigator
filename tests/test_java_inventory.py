"""Real-JVM checks for source-inventory failures and compiled-only resolution."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from test_java_context import LIB, installed_jdk, require_analyzer, run


class JavaInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        home = installed_jdk()
        cls.java = home / "bin/java"
        cls.javac = home / "bin/javac"
        require_analyzer()

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="CodeGraph inventory ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def compile(self, name, relative, content):
        source = self.root / name / "src" / relative
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text(content)
        classes = self.root / name / "classes"
        run(self.javac, "-g", "-d", classes, source)
        return source, classes

    def inventory(self, source, classes, classpath):
        return subprocess.run([str(self.java), "-cp", str(LIB / "*"),
            "codegraph.bytecode.SourceInventory", str(self.root / "inventory.tsv"),
            os.pathsep.join(map(str, classpath)), str(classes), str(source.parents[1])],
            capture_output=True, text=True)

    def test_unresolved_signature_fails_with_declaration_and_type(self):
        source, classes = self.compile("project", "fixture/Flow.java",
            "package fixture; public class Flow { public String missing() { return null; } }")
        source.write_text(
            "package fixture; public class Flow { public MissingType missing() { return null; } }")
        result = self.inventory(source, classes, [classes])
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cannot inventory fixture.Flow:missing()", result.stderr)
        self.assertIn("Cannot resolve source declaration type MissingType", result.stderr)

    def test_unresolved_name_does_not_bind_to_unrelated_default_package_type(self):
        source, classes = self.compile("project", "fixture/Flow.java",
            "package fixture; public class Flow { public String missing() { return null; } }")
        _, dependency = self.compile("dependency", "MissingType.java",
            "public class MissingType {}")
        source.write_text(
            "package fixture; public class Flow { public MissingType missing() { return null; } }")
        result = self.inventory(source, classes, [classes, dependency])
        self.assertNotEqual(result.returncode, 0,
            "A named-package signature cannot resolve a default-package class")
        self.assertIn("Cannot inventory fixture.Flow:missing()", result.stderr)
        self.assertIn("MissingType", result.stderr)

    def test_architecture_retains_source_annotations_and_interface_attributes(self):
        source, classes = self.compile("project", "fixture/Flow.java", """package fixture;
import java.lang.annotation.*;
@Retention(RetentionPolicy.SOURCE) @interface Layer {}
@Layer interface Contract { void run(); }
@Layer public class Flow implements Contract { @Layer public void run() {} }
record Data(String value) {}
""")
        result = self.inventory(source, classes, [classes])
        self.assertEqual(result.returncode, 0, result.stderr)
        rows = (self.root / "inventory.tsv").read_text().splitlines()
        self.assertTrue(rows[0].startswith("kind_v5\t"))
        row = next(row.split("\t") for row in rows if row.startswith("architecture\t"))
        self.assertEqual(Path(row[1]), source.resolve())
        self.assertIn("Flow", row[3].split(","))
        self.assertIn("Data", row[3].split(","))
        self.assertIn("record", row[4].split(","))
        self.assertIn("fixture.Layer", row[5].split(","))
        self.assertIn("Layer", row[6].split(","))
        self.assertIn("fixture.Contract", row[7].split(","))
        self.assertIn("Layer", row[8].split(","))

    def test_flow_inventory_uses_compiler_targets_and_utf8_source_offsets(self):
        source, classes = self.compile("project", "fixture/Flow.java", """package fixture;
public class Flow {
  static int leaf(int x) { return x; }
  static int leaf(String x) { return x.length(); }
  int run(int x) { String text = "é😀"; try { if (x > 0) return leaf(x); else throw new IllegalStateException(); } finally { leaf(0); } }
}
""")
        result = self.inventory(source, classes, [classes])
        self.assertEqual(result.returncode, 0, result.stderr)
        import json
        rows = (self.root / "inventory.tsv").read_text().splitlines()
        flow_rows = [row.split("\t") for row in rows if row.startswith("flow\t")]
        self.assertTrue(flow_rows, "Source inventory must export source flow IR")
        from test_java_context import tsv
        flows = [json.loads(row["start_line"]) for row in tsv(self.root / "inventory.tsv") if row["kind_v5"] == "flow"]
        flow = next(flow for flow in flows if flow["symbol"] == "fixture.Flow:run(int)|int")
        body = flow["body"]
        self.assertEqual(body[1]["kind"], "try")
        branch = body[1]["body"][0]
        self.assertEqual(branch["kind"], "if")
        call = branch["then_branch"][0]["value"]["items"][-1]["calls"][0]
        self.assertEqual(call["targets"][0]["symbol"], "fixture.Flow:leaf(int)|int")
        span = call["span"]
        self.assertEqual(source.read_bytes()[span["start"]:span["end"]].decode(), "leaf(x)")
        self.assertTrue(body[1]["finally"])

    def resolve(self, classpath, qualified_name):
        source = self.root / "probe/codegraph/bytecode/Resolve.java"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text("""package codegraph.bytecode;
import javax.tools.ToolProvider;
public class Resolve {
    public static void main(String[] args) throws Exception {
        try (var types = new CompiledTypes(ToolProvider.getSystemJavaCompiler(), args[0])) {
            System.out.println(types.binaryName(args[1]));
        }
    }
}
""")
        classes = self.root / "probe/classes"
        run(self.javac, "-cp", LIB / "*", "-d", classes, source)
        return subprocess.run([str(self.java), "-cp", os.pathsep.join([str(classes), str(LIB / "*")]),
            "codegraph.bytecode.Resolve", os.pathsep.join(map(str, classpath)), qualified_name],
            capture_output=True, text=True)

    def test_compiled_lookup_respects_first_classpath_definition(self):
        _, first = self.compile("first", "fixture/Outer.java",
            "package fixture; public class Outer { public static class Builder {} }")
        _, second = self.compile("second", "fixture/Outer.java",
            "package fixture; public class Outer {}")
        result = self.resolve([first, second], "fixture.Outer.Builder")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "fixture.Outer$Builder")
        result = self.resolve([second, first], "fixture.Outer.Builder")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("in compiled output", result.stderr)

    def test_compiled_lookup_does_not_load_sources_from_classpath(self):
        source = self.root / "dependency/fixture/Outer.java"
        source.parent.mkdir(parents=True)
        source.write_text("package fixture; public class Outer { public static class Builder {} }")
        result = self.resolve([source.parents[1]], "fixture.Outer.Builder")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cannot resolve source declaration type fixture.Outer.Builder", result.stderr)
        self.assertFalse(source.with_suffix(".class").exists())


if __name__ == "__main__":
    unittest.main()
