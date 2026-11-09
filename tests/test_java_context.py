"""Real-JVM regression checks for context staging and project body filtering."""

import csv
import io
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import unittest
import zipfile


ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "tools/java-bytecode/build/install/codegraph-bytecode/lib"


def run(*arguments):
    return subprocess.run([str(argument) for argument in arguments], check=True,
                          text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)


def tsv(path):
    rows = csv.DictReader(io.StringIO(path.read_text()), delimiter="\t")
    escapes = {"t": "\t", "n": "\n", "r": "\r"}
    return [{key: re.sub(r"\\(.)", lambda match: escapes.get(match[1], match[1]), value)
             for key, value in row.items()} for row in rows]


def status(output):
    return dict(line.split("=", 1) for line in
                (output / "analysis-status.properties").read_text().splitlines()
                if line and not line.startswith("#"))


def installed_jdk():
    home = os.environ.get("JAVA_HOME")
    if home:
        return Path(home)
    try:
        return Path(run("/usr/libexec/java_home", "-v", "25").stdout.strip())
    except (OSError, subprocess.CalledProcessError):
        raise unittest.SkipTest("JDK 25 is required for real Java context tests")


def require_analyzer():
    jar = LIB / "codegraph-bytecode.jar"
    if not jar.is_file():
        raise unittest.SkipTest("Build the pinned Java analyzer before running context tests")
    with zipfile.ZipFile(jar) as archive:
        if "codegraph/bytecode/ContextStager.class" not in archive.namelist():
            raise unittest.SkipTest("Rebuild the analyzer to include ContextStager")


class JavaContextTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        home = installed_jdk()
        cls.java = home / "bin/java"
        cls.javac = home / "bin/javac"
        if not cls.java.is_file() or not cls.javac.is_file():
            raise unittest.SkipTest("JAVA_HOME must identify an installed JDK")
        require_analyzer()

    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="CodeGraph Java context ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def compile(self, name, sources, classpath=None):
        root = self.root / name
        output = root / "classes"
        files = []
        for relative, content in sources.items():
            source = root / "src" / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text(content)
            files.append(source)
        arguments = [self.javac, "-g", "-d", output]
        if classpath:
            arguments.extend(["-classpath", classpath])
        run(*arguments, *files)
        return output

    def choice(self, name, value):
        return self.compile(name, {"sample/Choice.java":
            "package sample; public class Choice { public static int pick() { return "
            + str(value) + "; } }"})

    def stage(self, name, *entries):
        output = self.root / name
        run(self.java, "-cp", LIB / "*", "codegraph.bytecode.ContextStager", output, *entries)
        return output

    def archive(self, name, entries, multi_release=False):
        archive = self.root / name
        with zipfile.ZipFile(archive, "w") as jar:
            manifest = "Manifest-Version: 1.0\n"
            if multi_release:
                manifest += "Multi-Release: true\n"
            jar.writestr("META-INF/MANIFEST.MF", manifest + "\n")
            for relative, contents in entries.items():
                jar.writestr(relative, contents)
        return archive

    def test_first_definition_wins_and_shadow_bytes_and_escaped_origins_are_preserved(self):
        first = self.choice("first \twith\nescapes", 1)
        identical = self.root / "identical copy"
        shutil.copytree(first, identical)
        different = self.choice("different copy", 2)
        entry = "sample/Choice.class"
        archive = self.archive("different copy.jar", {entry: (different / entry).read_bytes()})

        output = self.stage("selected first", first, identical, archive)

        inputs = tsv(output / "inputs.tsv")
        self.assertEqual(len(inputs), 1)
        self.assertEqual(inputs[0]["original_path"], str(first))
        self.assertEqual((Path(inputs[0]["staged_path"]) / entry).read_bytes(), (first / entry).read_bytes())
        shadows = tsv(output / "shadows.tsv")
        self.assertEqual([row["identical"] for row in shadows], ["true", "false"])
        self.assertEqual([row["selected_path"] for row in shadows], [str(first), str(first)])
        self.assertEqual([row["shadowed_path"] for row in shadows], [str(identical), str(archive)])
        provenance = tsv(output / "provenance.tsv")
        self.assertEqual(provenance[0]["original_path"], str(first))
        self.assertEqual(provenance[0]["class_name"], "sample.Choice")
        self.assertEqual(provenance[0]["class_entry"], entry)

        reversed_output = self.stage("selected archive", archive, first)
        selected = tsv(reversed_output / "inputs.tsv")[0]
        self.assertEqual(selected["original_path"], str(archive))
        self.assertEqual((Path(selected["staged_path"]) / entry).read_bytes(), (different / entry).read_bytes())

    def test_filtered_staging_uses_external_visibility_without_staging_external_classes(self):
        entry = "sample/Choice.class"
        external = self.choice("external winner", 1)
        project = self.compile("project copies", {
            "sample/Choice.java": "package sample; public class Choice { public static int pick() { return 2; } }",
            "sample/Caller.java": "package sample; public class Caller { public int run() { return Choice.pick(); } }",
        })
        archive = self.archive("external winner.jar", {entry: (external / entry).read_bytes()})
        output = self.root / "filtered project winners"

        run(self.java, "-cp", LIB / "*", "codegraph.bytecode.ContextStager", output,
            "--project-inputs", "2", archive, project)

        inputs = tsv(output / "inputs.tsv")
        self.assertEqual(len(inputs), 1)
        self.assertEqual(inputs[0]["original_path"], str(project))
        selected = Path(inputs[0]["staged_path"])
        self.assertTrue((selected / "sample/Caller.class").is_file())
        self.assertFalse((selected / entry).exists())
        self.assertEqual(len(list(output.glob("input-*"))), 1)
        provenance = tsv(output / "provenance.tsv")
        self.assertEqual([row["class_name"] for row in provenance], ["sample.Caller"])
        self.assertTrue(all(row["original_path"] == str(project) for row in provenance))
        shadow = tsv(output / "shadows.tsv")
        self.assertEqual(len(shadow), 1)
        self.assertEqual(shadow[0]["class_name"], "sample.Choice")
        self.assertEqual(shadow[0]["selected_path"], str(archive))
        self.assertEqual(shadow[0]["shadowed_path"], str(project))
        self.assertEqual(shadow[0]["identical"], "false")

    def test_multi_release_archive_selects_supported_version_but_directory_uses_base(self):
        entry = "sample/Choice.class"
        base = (self.choice("base", 1) / entry).read_bytes()
        supported = (self.choice("version21", 21) / entry).read_bytes()
        future = (self.choice("version26", 26) / entry).read_bytes()
        contents = {entry: base, "META-INF/versions/21/" + entry: supported,
                    "META-INF/versions/26/" + entry: future}
        archive = self.archive("multi release.jar", contents, multi_release=True)
        output = self.stage("archive winners", archive)
        selected = Path(tsv(output / "inputs.tsv")[0]["staged_path"])
        self.assertEqual((selected / entry).read_bytes(), supported)
        self.assertEqual(tsv(output / "provenance.tsv")[0]["class_entry"], "META-INF/versions/21/" + entry)

        directory = self.root / "unpacked archive"
        for relative, data in contents.items():
            target = directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
        output = self.stage("directory winners", directory)
        selected = Path(tsv(output / "inputs.tsv")[0]["staged_path"])
        self.assertEqual((selected / entry).read_bytes(), base)
        self.assertEqual(tsv(output / "shadows.tsv"), [])

    def test_standalone_analyzer_filter_keeps_dependency_inheritance_and_project_overrides(self):
        external = self.compile("external", {
            "sample/Contract.java": "package sample; public interface Contract { void act(); }",
            "sample/Base.java": "package sample; public class Base { public void inherited() {} }",
        })
        project = self.compile("project", {"sample/Impl.java":
            "package sample; public class Impl extends Base implements Contract { "
            "public void act() { inherited(); } "
            "public boolean equals(Object value) { return this == value; } "
            "public int hashCode() { return 7; } }"}, external)
        staged = self.stage("engine inputs", project, external)
        inputs = [row["staged_path"] for row in tsv(staged / "inputs.tsv")]
        output = self.root / "engine output"

        run(self.java, "-Xmx1g", "-cp", LIB / "*", "codegraph.bytecode.Analyzer",
            output, "25", "--project-inputs", "1", *inputs)

        properties = status(output)
        self.assertEqual(properties["analysisComplete"], "true")
        raw = Path(properties["outputDirectory"])
        declarations = (raw / "method_info.txt").read_text()
        self.assertIn("sample.Impl:act()", declarations)
        self.assertIn("sample.Impl:equals(java.lang.Object)", declarations)
        self.assertIn("sample.Impl:hashCode()", declarations)
        self.assertNotIn("sample.Base:inherited()", declarations)
        self.assertNotIn("sample.Contract:act()", declarations)
        calls = (raw / "method_call.txt").read_text()
        self.assertIn("sample.Impl:act()\t", calls)
        self.assertIn("(VIR)sample.Impl:inherited()", calls)
        self.assertIn("(_CCS)sample.Base:inherited()", calls)
        self.assertIn("(_ITF)sample.Impl:act()", calls)

    def test_filtered_marker_context_accepts_zero_methods_but_legacy_cli_rejects_it(self):
        marker = self.compile("marker project", {
            "sample/Marker.java": "package sample; public interface Marker {}",
        })
        external = self.compile("external methods", {
            "sample/External.java": "package sample; public class External { public void work() {} }",
        })
        staged = self.stage("marker context", marker, external)
        inputs = [row["staged_path"] for row in tsv(staged / "inputs.tsv")]
        output = self.root / "filtered marker output"

        run(self.java, "-Xmx1g", "-cp", LIB / "*", "codegraph.bytecode.Analyzer",
            output, "25", "--project-inputs", "1", *inputs)

        properties = status(output)
        self.assertEqual(properties["engineSucceeded"], "true")
        self.assertEqual(properties["analysisComplete"], "true")
        declarations = Path(properties["outputDirectory"]) / "method_info.txt"
        self.assertEqual(declarations.read_text(), "")
        metadata = tsv(output / "method_metadata.tsv")
        self.assertTrue(metadata)
        self.assertTrue(all(row["class_name"] == "sample.External" for row in metadata))

        legacy_output = self.root / "legacy marker output"
        with self.assertRaises(subprocess.CalledProcessError) as error:
            run(self.java, "-Xmx1g", "-cp", LIB / "*", "codegraph.bytecode.Analyzer",
                legacy_output, "25", marker)
        self.assertEqual(error.exception.returncode, 1)
        self.assertEqual(status(legacy_output)["analysisComplete"], "false")


if __name__ == "__main__":
    unittest.main()
