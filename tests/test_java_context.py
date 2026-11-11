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
LIB = ROOT / "tools/java-bytecode/build/install/codecallnavigator-bytecode/lib"


def run(*arguments, timeout=None):
    return subprocess.run([str(argument) for argument in arguments], check=True,
                          text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)


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
    jar = LIB / "codecallnavigator-bytecode.jar"
    if not jar.is_file():
        raise unittest.SkipTest("Build the pinned Java analyzer before running context tests")
    with zipfile.ZipFile(jar) as archive:
        if "codecallnavigator/bytecode/ContextStager.class" not in archive.namelist():
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
        temporary = tempfile.TemporaryDirectory(prefix="CodeCallNavigator Java context ")
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
        run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", output, *entries)
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

        run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", output,
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

    def test_job_inventory_reuses_artifacts_without_changing_context_order(self):
        first = self.choice("cached first", 1)
        second = self.choice("cached second", 2)
        cache = self.root / "job artifact inventories"
        entry = "sample/Choice.class"
        outputs = []
        for name, entries in [("initial", [first, second]), ("reversed", [second, first])]:
            output = self.root / name
            run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", output,
                "--project-inputs", "1,2", "--artifact-cache", cache, *entries)
            outputs.append(output)
            selected = Path(tsv(output / "inputs.tsv")[0]["staged_path"])
            self.assertEqual((selected / entry).read_bytes(), (entries[0] / entry).read_bytes())
            shadow = tsv(output / "shadows.tsv")[0]
            self.assertEqual(shadow["selected_path"], str(entries[0]))
            self.assertEqual(shadow["shadowed_path"], str(entries[1]))
            self.assertEqual(shadow["identical"], "false")
        statistics = [dict(line.split("=", 1) for line in
                          (output / "artifact-stats.properties").read_text().splitlines()) for output in outputs]
        self.assertEqual(statistics[0]["artifact_reads"], "2")
        self.assertEqual(statistics[1]["artifact_reads"], "0")
        self.assertEqual(statistics[1]["artifact_hits"], "2")
        self.assertEqual(statistics[1]["classes_indexed"], "0")
        self.assertEqual(statistics[1]["artifact_bytes_read"], "0")
        self.assertEqual(statistics[1]["artifact_retained_bytes"], "0")

    def test_prefetch_duplicate_inputs_remain_ordered_and_are_indexed_once(self):
        first = self.choice("prefetched first", 1)
        second = self.choice("prefetched second", 2)
        entry = "sample/Choice.class"
        for iteration in range(3):
            output = self.root / f"prefetch-{iteration}"
            run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", output,
                first, first, second)
            selected = Path(tsv(output / "inputs.tsv")[0]["staged_path"])
            self.assertEqual((selected / entry).read_bytes(), (first / entry).read_bytes())
            shadows = tsv(output / "shadows.tsv")
            self.assertEqual([row["identical"] for row in shadows], ["true", "false"])
            self.assertEqual([row["selected_path"] for row in shadows], [str(first), str(first)])
            statistics = dict(line.split("=", 1) for line in
                              (output / "artifact-stats.properties").read_text().splitlines())
            self.assertEqual(statistics["artifact_reads"], "2")
            self.assertEqual(statistics["artifact_hits"], "1")
            self.assertEqual(statistics["classes_indexed"], "2")
            self.assertEqual(int(statistics["artifact_bytes_read"]),
                             (first / entry).stat().st_size + (second / entry).stat().st_size)

    def test_shared_inventory_can_promote_external_artifact_without_reindexing(self):
        classes = self.choice("shared dependency", 1)
        archive = self.archive("shared dependency.jar", {
            "sample/Choice.class": (classes / "sample/Choice.class").read_bytes()})
        project = self.choice("other project", 2)
        cache = self.root / "shared cache"
        external_output = self.root / "external context"
        run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", external_output,
            "--project-inputs", "1", "--artifact-cache", cache, project, archive)
        project_output = self.root / "project context"
        run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", project_output,
            "--project-inputs", "1", "--artifact-cache", cache, archive)
        selected = Path(tsv(project_output / "inputs.tsv")[0]["staged_path"])
        self.assertEqual((selected / "sample/Choice.class").read_bytes(), (classes / "sample/Choice.class").read_bytes())
        statistics = dict(line.split("=", 1) for line in
                          (project_output / "artifact-stats.properties").read_text().splitlines())
        self.assertEqual(statistics["artifact_reads"], "0")
        self.assertEqual(statistics["artifact_hits"], "1")
        self.assertEqual(statistics["classes_indexed"], "0")
        self.assertGreater(int(statistics["artifact_retained_bytes"]), 0)

    def test_failed_artifact_inventory_does_not_poison_the_job_cache(self):
        classes = self.root / "invalid/classes"
        classes.mkdir(parents=True)
        (classes / "Invalid.class").write_bytes(b"not bytecode")
        cache = self.root / "job inventory"
        with self.assertRaises(subprocess.CalledProcessError):
            run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", self.root / "failed",
                "--project-inputs", "1", "--artifact-cache", cache, classes)
        self.assertEqual(list(cache.iterdir()), [])
        shutil.rmtree(classes)
        valid = self.choice("valid retry", 3)
        run(self.java, "-cp", LIB / "*", "codecallnavigator.bytecode.ContextStager", self.root / "retry",
            "--project-inputs", "1", "--artifact-cache", cache, valid)
        self.assertEqual(len(list(cache.iterdir())), 1)

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

        run(self.java, "-Xmx1g", "-cp", LIB / "*", "codecallnavigator.bytecode.Analyzer",
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

    def analyze_classes(self, classes):
        output = self.root / "inference output"
        run(self.java, "-Xmx1g", "-cp", LIB / "*", "codecallnavigator.bytecode.Analyzer",
            output, "25", "--project-inputs", "1", classes, timeout=30)
        properties = status(output)
        self.assertEqual(properties["analysisComplete"], "true")
        self.assertEqual(properties["frameSnapshotLimitMethods"], "0")
        return Path(properties["outputDirectory"])

    def test_only_object_equality_signatures_skip_inference_and_keep_bytecode_calls(self):
        classes = self.compile("equality policy", {"sample/Equality.java": """
            package sample;
            interface Action { int act(); }
            class Concrete implements Action { public int act() { return 7; } }
            public class Equality {
                static int marker() { return 1; }
                public boolean equals(Object other) {
                    Action action = new Concrete(); return action.act() == marker();
                }
                public int hashCode() {
                    Action action = new Concrete(); return action.act() + marker();
                }
                public boolean equals(String other) {
                    Action action = new Concrete(); return action.act() == 7;
                }
                public int hashCode(int seed) {
                    Action action = new Concrete(); return action.act() + seed;
                }
                public int ordinary() {
                    Action action = new Concrete(); return action.act();
                }
                public boolean notEquals(Object other) {
                    Action action = new Concrete(); return action.act() == 7;
                }
            }
        """})
        raw = self.analyze_classes(classes)
        declarations = (raw / "method_info.txt").read_text()
        calls = [line.split("\t") for line in (raw / "method_call.txt").read_text().splitlines()]
        for signature in ["equals(java.lang.Object)", "hashCode()"]:
            caller = "sample.Equality:" + signature
            self.assertIn(caller, declarations)
            targets = [row[3] for row in calls if row[2] == caller]
            self.assertIn("(STA)sample.Equality:marker()", targets)
            self.assertIn("(INT)sample.Action:act()", targets)
            self.assertNotIn("(_ACT_I)sample.Concrete:act()", targets)
        for signature in ["equals(java.lang.String)", "hashCode(int)", "ordinary()", "notEquals(java.lang.Object)"]:
            targets = [row[3] for row in calls if row[2] == "sample.Equality:" + signature]
            self.assertIn("(_ACT_I)sample.Concrete:act()", targets, signature)

    def test_large_lombok_equality_methods_finish_without_snapshot_limits(self):
        lombok = ROOT / "vendor/lombok-1.18.48.jar"
        if not lombok.is_file():
            self.skipTest("Pinned Lombok jar is required")
        source = self.root / "LargeValue.java"
        fields = "\n".join(f"private Object field{index};" for index in range(40))
        source.write_text("package sample; @lombok.Data public class LargeValue {" + fields + "}")
        classes = self.root / "large classes"
        run(self.javac, "-g", "-cp", lombok, "-processorpath", lombok,
            "-processor", "lombok.launch.AnnotationProcessorHider$AnnotationProcessor",
            "-d", classes, source)
        raw = self.analyze_classes(classes)
        declarations = (raw / "method_info.txt").read_text()
        self.assertIn("sample.LargeValue:equals(java.lang.Object)", declarations)
        self.assertIn("sample.LargeValue:hashCode()", declarations)
        calls = (raw / "method_call.txt").read_text()
        self.assertIn("sample.LargeValue:equals(java.lang.Object)\t", calls)
        self.assertIn("sample.LargeValue:hashCode()\t", calls)

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

        run(self.java, "-Xmx1g", "-cp", LIB / "*", "codecallnavigator.bytecode.Analyzer",
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
            run(self.java, "-Xmx1g", "-cp", LIB / "*", "codecallnavigator.bytecode.Analyzer",
                legacy_output, "25", marker)
        self.assertEqual(error.exception.returncode, 1)
        self.assertEqual(status(legacy_output)["analysisComplete"], "false")


if __name__ == "__main__":
    unittest.main()
