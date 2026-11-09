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
