"""Verification orchestration against real Git, subprocess and SQLite boundaries."""
import contextlib
import io
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import verify
from verification import checks, prerequisites, profile, quality
from verification.runner import Runner, Step, exclusive
from pan_profile import snapshot_store


class GitChanges(unittest.TestCase):
    def test_staged_unstaged_untracked_deleted_and_vendor_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True)
            git("init", "-q")
            (root / ".gitignore").write_text("ignored.rs\n")
            for name in ["staged.rs", "unstaged.rs", "deleted.rs", "vendor/upstream.rs"]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("original\n")
            git("add", ".")
            git("-c", "user.name=Verification Test", "-c", "user.email=verify@example.invalid", "commit", "-qm", "initial")
            (root / "staged.rs").write_text("staged\n")
            git("add", "staged.rs")
            (root / "unstaged.rs").write_text("unstaged\n")
            (root / "deleted.rs").unlink()
            (root / "vendor/upstream.rs").write_text("changed upstream\n")
            unusual = "untracked space\nfile.rs"
            (root / unusual).write_text("fn main() {}\n")
            (root / "vendor/untracked.rs").write_text("excluded\n")
            (root / "ignored.rs").write_text("excluded generated file\n")
            paths = checks.changed_files(root, "HEAD")
            self.assertEqual(paths, sorted(["staged.rs", "unstaged.rs", "deleted.rs", unusual]))
            steps = checks.static_steps(root, paths, "HEAD", sys.executable)
            rust = [step.command for step in steps if step.command[0] == "rustfmt"]
            self.assertEqual({command[-1] for command in rust}, {"staged.rs", "unstaged.rs", unusual})
            self.assertTrue(all("skip_children=true" in command for command in rust))
            self.assertTrue(all("--check" in command for command in rust))

    def test_docs_classifier_is_conservative_about_executable_inputs(self):
        self.assertTrue(checks.docs_only(["README.md", "docs/reference.md", ".backlog/tasks/task-1.md", "LICENSE"]))
        for path in ["Cargo.toml", "Cargo.lock", "tools/bootstrap.py", "verify-full.sh", "docs/runtime.py", ".github/workflows/ci.yml", "fixtures/App.csproj"]:
            with self.subTest(path=path):
                self.assertFalse(checks.docs_only(["README.md", path]))


class SubprocessStages(unittest.TestCase):
    def run_steps(self, root, steps):
        output = root / "evidence"
        with contextlib.redirect_stdout(io.StringIO()):
            code = Runner(root, output, steps).run()
        return code, json.loads((output / "results.json").read_text())

    def test_failure_preserves_logs_and_does_not_run_later_stages(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            steps = [Step("success", [sys.executable, "-c", "print('first completed')"]),
                     Step("failure", [sys.executable, "-c", "print('failure detail'); raise SystemExit(7)"]),
                     Step("later", [sys.executable, "-c", "open('should-not-exist', 'w').write('bad')"]),
                     Step("cleanup", [sys.executable, "-c", "open('cleanup-ran', 'w').write('done')"], always=True)]
            code, results = self.run_steps(root, steps)
            self.assertEqual(code, 1)
            self.assertEqual([r["status"] for r in results], ["passed", "failed", "not-run", "passed"])
            self.assertEqual(results[1]["exit_code"], 7)
            self.assertIn("failure detail", (root / "evidence" / results[1]["log"]).read_text())
            self.assertFalse((root / "should-not-exist").exists())
            self.assertEqual((root / "cleanup-ran").read_text(), "done")
            with self.assertRaises(FileExistsError):
                Runner(root, root / "evidence", steps).run()

    def test_cleanup_failure_fails_an_otherwise_successful_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            steps = [Step("success", [sys.executable, "-c", "print('tests passed')"]),
                     Step("cleanup", [sys.executable, "-c", "print('cleanup failed'); raise SystemExit(8)"], always=True)]
            code, results = self.run_steps(root, steps)
            self.assertEqual(code, 1)
            self.assertEqual([r["status"] for r in results], ["passed", "failed"])
            self.assertEqual(results[1]["exit_code"], 8)
            self.assertIn("cleanup failed", (root / "evidence" / results[1]["log"]).read_text())

    def test_active_stage_publishes_running_before_subprocess_executes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            command = (
                "import json,pathlib; "
                "rows=json.loads(pathlib.Path('evidence/results.json').read_text()); "
                "assert rows[0]['status']=='running', rows; "
                "assert rows[1]['status']=='not-run', rows; "
                "print('observed running state')"
            )
            code, results = self.run_steps(root, [
                Step("active", [sys.executable, "-c", command]),
                Step("next", [sys.executable, "-c", "pass"]),
            ])
            self.assertEqual(code, 0)
            self.assertEqual([row["status"] for row in results], ["passed", "passed"])
            self.assertIn("observed running state", (root / "evidence" / results[0]["log"]).read_text())

    def test_timeout_terminates_descendants_and_records_remaining_not_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            child = "import time; time.sleep(1); open('orphan-side-effect', 'w').write('alive')"
            parent = f"import os,subprocess,sys,time; open('parent.pid','w').write(str(os.getpid())); subprocess.Popen([sys.executable, '-c', {child!r}]); print('child launched',flush=True); time.sleep(30)"
            code, results = self.run_steps(root, [Step("timeout", [sys.executable, "-c", parent], timeout=0.2), Step("later", [sys.executable, "-c", "pass"])])
            self.assertEqual(code, 1)
            self.assertIn(results[0]["status"], ["timed-out", "cleanup-unavailable"])
            self.assertEqual(results[1]["status"], "not-run")
            log = (root / "evidence" / results[0]["log"]).read_text()
            self.assertIn("timed out", log)
            with self.assertRaises(ProcessLookupError):
                os.kill(int((root / "parent.pid").read_text()), 0)
            time.sleep(1.1)
            if results[0]["status"] == "timed-out":
                self.assertFalse((root / "orphan-side-effect").exists(), "timed-out stage left a running descendant")
            else:
                self.assertIn("descendant", log.lower())
                self.assertIn("unverified", log.lower())

    def test_denied_group_cleanup_is_reported_as_unavailable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch("verification.runner.os.killpg", side_effect=PermissionError("group signaling denied")):
                code, results = self.run_steps(root, [Step("timeout", [sys.executable, "-c", "import time; time.sleep(30)"], timeout=0.1)])
            self.assertEqual(code, 1)
            self.assertEqual(results[0]["status"], "cleanup-unavailable")
            self.assertIsNotNone(results[0]["exit_code"])
            log = (root / "evidence" / results[0]["log"]).read_text()
            self.assertIn("unverified", log.lower())

    def test_missing_executable_is_a_failed_stage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            code, results = self.run_steps(root, [Step("missing", [str(root / "missing-command")])])
            self.assertEqual(code, 1)
            self.assertEqual(results[0]["status"], "failed")
            self.assertIsNone(results[0]["exit_code"])

    def test_checkout_lock_excludes_competing_process_and_releases_after_close(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            child = (
                f"import sys; sys.path.insert(0, {str(ROOT / 'tools')!r})\n"
                "from verification.runner import exclusive\n"
                "try:\n"
                "    with exclusive(sys.argv[1]):\n"
                "        print('acquired')\n"
                "except RuntimeError:\n"
                "    sys.exit(19)\n"
            )
            command = [sys.executable, "-c", child, str(root)]
            with exclusive(root):
                with self.assertRaisesRegex(RuntimeError, "Another verification"):
                    with exclusive(root):
                        self.fail("same-checkout verification acquired a held lock")
                blocked = subprocess.run(command, capture_output=True, text=True, timeout=5)
                self.assertEqual(blocked.returncode, 19)
                self.assertNotIn("acquired", blocked.stdout)
            released = subprocess.run(command, capture_output=True, text=True, timeout=5)
            self.assertEqual(released.returncode, 0, released.stderr)
            self.assertEqual(released.stdout.strip(), "acquired")


def profiling_source(root):
    root.mkdir()
    connection = sqlite3.connect(root / "catalog.sqlite3")
    connection.execute("PRAGMA journal_mode=WAL")
    connection.execute("CREATE TABLE catalog (key TEXT PRIMARY KEY, value TEXT)")
    connection.executemany("INSERT INTO catalog VALUES (?, ?)", [("active", "generation.sqlite3"), ("session", json.dumps({"navigation": {}}))])
    connection.commit()
    with contextlib.closing(sqlite3.connect(root / "generation.sqlite3")) as graph, graph:
        graph.execute("CREATE TABLE edges (id INTEGER)")
        graph.execute("INSERT INTO edges VALUES (1)")
    return connection


class ProfilingSnapshots(unittest.TestCase):
    def test_cli_cache_mismatch_is_rejected_before_copying_graph(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            source=root/'live'
            connection=profiling_source(source)
            try:
                session={'navigation':{},'graph_context':{'generation':'a'*64},
                         'presentation_cache':{'source_generation':'b'*64,'cache_generation':'generation.sqlite3'}}
                connection.execute("UPDATE catalog SET value=? WHERE key='session'",(json.dumps(session),))
                connection.commit()
                with self.assertRaisesRegex(ValueError,'cache'):
                    profile.validate(source)
                with self.assertRaisesRegex(ValueError,'cache'):
                    snapshot_store(source,root/'rejected')
                self.assertFalse((root/'rejected/generation.sqlite3').exists())
            finally:
                connection.close()

    def test_coherent_cli_cache_and_context_are_copied_together(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            source,destination=root/'live',root/'snapshot'
            connection=profiling_source(source)
            try:
                session={'navigation':{},'graph_context':{'generation':'a'*64},
                         'presentation_cache':{'source_generation':'a'*64,'cache_generation':'generation.sqlite3'}}
                connection.execute("UPDATE catalog SET value=? WHERE key='session'",(json.dumps(session),))
                connection.commit()
                profile.validate(source)
                snapshot_store(source,destination)
                profile.validate(destination)
                connection.execute("UPDATE catalog SET value='different.sqlite3' WHERE key='active'")
                connection.commit()
                with self.assertRaisesRegex(ValueError,'cache'):
                    profile.validate(source)
                profile.validate(destination)
                self.assertEqual((source/'generation.sqlite3').read_bytes(),(destination/'generation.sqlite3').read_bytes())
            finally:
                connection.close()

    def test_backup_includes_committed_wal_and_isolates_mutations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, destination = root / "live", root / "snapshot"
            connection = profiling_source(source)
            try:
                connection.execute("INSERT INTO catalog VALUES ('wal-entry', 'committed')")
                connection.commit()
                self.assertTrue((source / "catalog.sqlite3-wal").is_file())
                profile.validate(source)
                snapshot_store(source, destination)
                profile.validate(destination)
                with contextlib.closing(sqlite3.connect(destination / "catalog.sqlite3")) as copied, copied:
                    self.assertEqual(copied.execute("SELECT value FROM catalog WHERE key='wal-entry'").fetchone(), ("committed",))
                    copied.execute("UPDATE catalog SET value='snapshot only' WHERE key='wal-entry'")
                self.assertEqual(connection.execute("SELECT value FROM catalog WHERE key='wal-entry'").fetchone(), ("committed",))
                with contextlib.closing(sqlite3.connect(destination / "generation.sqlite3")) as copied, copied:
                    copied.execute("DELETE FROM edges")
                with contextlib.closing(sqlite3.connect(source / "generation.sqlite3")) as original, original:
                    self.assertEqual(original.execute("SELECT COUNT(*) FROM edges").fetchone()[0], 1)
            finally:
                connection.close()

    def test_invalid_source_is_rejected_without_creating_databases(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(ValueError):
                profile.validate(root)
            self.assertFalse((root / "catalog.sqlite3").exists())
            connection = profiling_source(root / "source")
            try:
                connection.execute("UPDATE catalog SET value='../escape.sqlite3' WHERE key='active'")
                connection.commit()
                with self.assertRaises(ValueError):
                    profile.validate(root / "source")
                self.assertFalse((root / "escape.sqlite3").exists())
            finally:
                connection.close()


class VerificationPlans(unittest.TestCase):
    def test_current_toolchains_are_recognized_without_accepting_unreviewed_versions(self):
        self.assertTrue(prerequisites.dotnet_supported("11.0.100 [/sdk]"))
        self.assertFalse(prerequisites.dotnet_supported("10.0.404 [/sdk]"))
        self.assertFalse(prerequisites.dotnet_supported("unrecognized"))
        self.assertTrue(prerequisites.graphviz_supported("dot - graphviz version 16.1.1 (20261108.1752)", "16.1.1"))
        self.assertFalse(prerequisites.graphviz_supported("dot - graphviz version 16.1.0", "16.1.1"))
        self.assertFalse(prerequisites.graphviz_supported("dot - graphviz version 16.1.10", "16.1.1"))

    def args(self, mode, source=None):
        return SimpleNamespace(mode=mode, base="HEAD", timeout=123, profile_source=source, codescene_command="cs-mcp")

    def test_docs_light_has_no_runtime_and_runtime_light_has_no_packaging(self):
        docs = verify.plan(self.args("light"), ["README.md"], ROOT / "target/test-evidence")
        self.assertEqual([step.name for step in docs], ["whitespace", "cleanup"])
        self.assertTrue(docs[-1].always)
        runtime = verify.plan(self.args("light"), ["src/app.rs"], ROOT / "target/test-evidence")
        names = [step.name for step in runtime]
        self.assertEqual(names[0], "prerequisites")
        self.assertIn("--light", runtime[0].command)
        self.assertIn("unit-test-rust", names)
        self.assertIn("integration-test-rust", names)
        self.assertNotIn("acceptance-prepare", names)
        self.assertNotIn("performance-prepare", names)
        self.assertIn("codescene", names)
        self.assertNotIn("package", names)
        self.assertFalse(any("--include-ignored" in step.command for step in runtime))

    def test_quality_step_passes_all_existing_sources_to_language_filter(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            paths = ["Example.java", "Example.cs", "Widget.jsx", "README.md", "deleted.java"]
            for path in paths[:-1]:
                (root / path).write_text("source fixture\n")
            with patch.object(verify, "ROOT", root):
                step = verify.quality_step(self.args("light"), paths)
            arguments = step.command[step.command.index("--files") + 1:]
            self.assertEqual(arguments, paths[:-1])
            files, skipped = quality.source_files(root, arguments)
            self.assertEqual(files, ["Example.cs", "Example.java", "Widget.jsx"])
            self.assertEqual(skipped, ["README.md"])

    def test_python_dependency_preflight_reports_missing_and_wrong_pins(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            installed = root / "verification_fixture_installed-1.2.3.dist-info"
            installed.mkdir()
            (installed / "METADATA").write_text("Metadata-Version: 2.1\nName: verification-fixture-installed\nVersion: 1.2.3\n")
            requirements = root / "requirements-verification.txt"
            requirements.write_text("verification-fixture-installed==1.2.3 \\\n    --hash=sha256:deadbeef\nverification-fixture-missing==3.2.1\n")
            with patch.object(sys, "path", [str(root), *sys.path]):
                errors = prerequisites.python_problems(root)
                self.assertEqual(len(errors), 1)
                self.assertIn("verification-fixture-missing==3.2.1: not installed", errors[0])
                requirements.write_text("verification-fixture-installed==2.0.0\n")
                errors = prerequisites.python_problems(root)
                self.assertEqual(len(errors), 1)
                self.assertIn("verification-fixture-installed==2.0.0: 1.2.3", errors[0])

    def test_full_plan_has_preflight_snapshot_release_env_and_package(self):
        source = Path("/explicit-profile-source")
        output = ROOT / "target/test-evidence"
        steps = verify.plan(self.args("full", source), ["README.md"], output)
        names = [step.name for step in steps]
        self.assertEqual(names[0], "prerequisites")
        preflight = steps[0].command
        self.assertEqual(preflight[preflight.index("--profile-source") + 1], str(source))
        self.assertEqual(len(names), len(set(names)), "Composite stages must not execute twice")
        from verification.inventory import load_gates
        for gate in load_gates(ROOT):
            self.assertEqual(names.count(gate['category'] + '-gate-' + gate['test_name']), 1)
        self.assertLess(names.index("cargo-check"), names.index("mcp-dependency-boundary"))
        for category in ("unit", "integration", "acceptance", "performance"):
            cleanup = steps[names.index(f"{category}-category-cleanup")]
            self.assertTrue(cleanup.always)
            self.assertLess(names.index(f"{category}-prepare"), names.index(cleanup.name))
        for step in [step for step in steps if step.name.endswith('test-rust')]:
            category = step.name.split('-')[0]
            build = output / category / 'build'
            self.assertEqual(step.environment['CARGO_TARGET_DIR'], str(build))
            self.assertEqual(step.environment['CARGO_BUILD_BUILD_DIR'], str(build))
            self.assertEqual(step.environment['DOTNET_CLI_USE_MSBUILD_SERVER'], '0')
            self.assertEqual(step.environment['MSBUILDDISABLENODEREUSE'], '1')
            self.assertEqual(step.environment['UseSharedCompilation'], 'false')
            self.assertEqual(step.environment['CARGO_CACHE_AUTO_CLEAN_FREQUENCY'], 'never')
            self.assertEqual(step.environment['CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE'],
                             step.environment['CODECALLNAVIGATOR_TEST_ENGINE'])
            store = 'profile' if category == 'performance' else 'test-store'
            self.assertEqual(step.environment['CODECALLNAVIGATOR_DATA_DIR'], str(output / category / store))
        self.assertFalse(any("--include-ignored" in step.command for step in steps))
        self.assertNotIn("release-acceptance-and-benchmarks", names)
        snapshot = steps[names.index("performance-profile-snapshot")]
        self.assertEqual(snapshot.command[-2], str(source))
        self.assertLess(names.index(snapshot.name), names.index('performance-test-rust'))
        self.assertLess(names.index(snapshot.name), names.index("performance-gate-benchmark"))
        self.assertLess(names.index("acceptance-package"), names.index("acceptance-packaged-engine"))
        self.assertLess(names.index("acceptance-packaged-engine"), names.index("acceptance-mcp-independent-install"))
        installed = steps[names.index("acceptance-gate-installed_cli_acceptance")]
        self.assertIn(str(ROOT / "dist/CodeCallNavigatorEngine"), installed.command)
        self.assertIn(str(output / "acceptance/gate-installed_cli_acceptance"), installed.command)
        mcp = steps[names.index("acceptance-mcp-independent-install")]
        self.assertEqual(mcp.command[-1], str(ROOT / "dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine"))
        journey = steps[names.index("acceptance-gate-agent_journeys")]
        self.assertLess(names.index("acceptance-packaged-engine"), names.index(journey.name))
        self.assertEqual(journey.environment["PATH"].split(os.pathsep)[0], str(ROOT / "dist/CodeCallNavigatorEngine/bin"))
        self.assertNotIn("--engine", journey.command)
        packaged = steps[names.index("acceptance-agent-integrations-package")]
        self.assertEqual(packaged.command[-1], str(output / "acceptance/agent-integrations"))
        self.assertEqual(packaged.command[packaged.command.index("--host") + 1], "all")
        native = steps[names.index("acceptance-gate-agent_mcp_journeys")]
        self.assertLess(names.index("acceptance-mcp-independent-install"), names.index(native.name))
        self.assertEqual(native.environment["PATH"].split(os.pathsep)[:2], [
            str(ROOT / "dist/CodeCallNavigatorEngine/bin"), str(ROOT / "dist/CodeCallNavigatorMcp/bin")])
        self.assertNotIn("--engine", native.command)
        self.assertNotIn("--mcp", native.command)
        self.assertEqual(native.command[-1], str(output / "acceptance/gate-agent_mcp_journeys"))
        acceptance = steps[names.index("acceptance-test-rust")]
        self.assertEqual(acceptance.environment["CODECALLNAVIGATOR_ENGINE"],
                         str(output / "acceptance/build/release/codecallnavigator-engine"))
        self.assertEqual(acceptance.environment["CODECALLNAVIGATOR_TEST_ENGINE"], acceptance.environment["CODECALLNAVIGATOR_ENGINE"])
        regular = steps[names.index("unit-test-rust")]
        self.assertEqual(regular.environment["CODECALLNAVIGATOR_ENGINE"],
                         str(output / "unit/build/debug/codecallnavigator-engine"))

    def test_full_cleanup_acceptance_uses_built_engine_and_run_evidence(self):
        output = ROOT / "target/test-evidence"
        steps = verify.plan(self.args("full", Path("/profile-source")), ["README.md"], output)
        names = [step.name for step in steps]
        position = names.index("acceptance-gate-csharp_cleanup_acceptance")
        self.assertLess(names.index("acceptance-compile-cli"), position)
        self.assertLess(position, names.index("acceptance-category-cleanup"))
        acceptance = steps[position]
        self.assertEqual(acceptance.command[:2], [sys.executable, str(ROOT / "tools/csharp_cleanup_acceptance.py")])
        self.assertEqual(acceptance.command[acceptance.command.index("--engine") + 1],
                         str(output / "acceptance/build/release/codecallnavigator-engine"))
        self.assertEqual(acceptance.command[acceptance.command.index("--output") + 1],
                         str(output / "acceptance/gate-csharp_cleanup_acceptance"))
        self.assertEqual(acceptance.timeout, self.args("full").timeout)

    def test_full_dependency_preflight_accepts_complete_inputs_and_rejects_wrong_versions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            modules = root / "node_modules"
            for name, version in [("typescript", "6.0.2"), ("react", "19.1.0"), ("next", "16.1.0")]:
                (modules / name).mkdir(parents=True)
                (modules / name / "package.json").write_text(json.dumps({"version": version}))
            private = root / "private"
            private.mkdir()
            (private / "pom.xml").write_text("<project />")
            (root / "fixtures/gradle").mkdir(parents=True)
            (root / "fixtures/gradle/gradlew").write_text("#!/bin/sh\n")
            environment = {"CODECALLNAVIGATOR_TS_NODE_MODULES": str(modules), "CODECALLNAVIGATOR_RECURRING_BILLING": str(private)}
            self.assertEqual(prerequisites.dependencies(root, environment), [])
            (modules / "typescript/package.json").write_text('{"version": "5.9.3"}')
            self.assertEqual(prerequisites.dependencies(root, environment), [])
            (modules / "typescript/package.json").write_text('{"version": "5.8.3"}')
            self.assertTrue(any("typescript" in error for error in prerequisites.dependencies(root, environment)))

    def test_full_preflight_reports_independent_missing_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            errors = prerequisites.dependencies(Path(directory), {})
            self.assertTrue(any("NODE_MODULES" in error for error in errors))
            self.assertTrue(any("RECURRING_BILLING" in error for error in errors))
            self.assertTrue(any("gradlew" in error for error in errors))
            self.assertTrue(prerequisites.profile_problems(None))

    def test_wrappers_help_and_external_cwd_dry_run(self):
        with tempfile.TemporaryDirectory() as directory:
            for mode in ["full", "light", "unit", "integration", "acceptance", "performance",
                         "ui", "core", "engine", "mcp", "tooling"]:
                result = subprocess.run(["bash", str(ROOT / f"verify-{mode}.sh"), "--help"], cwd=directory,
                    env=os.environ | {"PYTHON": sys.executable}, capture_output=True, text=True, check=True)
                self.assertIn("--dry-run", result.stdout)
                if mode not in ("full", "light"):
                    leaf_output = Path(directory) / ("unused-" + mode)
                    selection = (["--component", "core"] if mode in ("unit", "integration", "acceptance", "performance")
                                 else ["--suite", "unit"])
                    dry = subprocess.run(["bash", str(ROOT / f"verify-{mode}.sh"), "--dry-run",
                                          "--output", str(leaf_output), *selection], cwd=directory,
                                         env=os.environ | {"PYTHON": sys.executable}, capture_output=True,
                                         text=True, check=True, timeout=60)
                    leaf_steps = json.loads(dry.stdout)
                    self.assertTrue(leaf_steps)
                    self.assertTrue(leaf_steps[-1]["always"])
                    self.assertTrue(leaf_steps[-1]["name"].endswith("category-cleanup"))
                    self.assertFalse(leaf_output.exists())
            scoped = subprocess.run(
                [sys.executable, str(ROOT / "tools/verify.py"), "integration", "--component", "engine", "--dry-run"],
                cwd=directory, env=os.environ | {"CODECALLNAVIGATOR_TS_NODE_MODULES": str(Path(directory) / "custom-modules"),
                                               "CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT": str(Path(directory) / "explicit-typescript")},
                capture_output=True, text=True, check=True)
            scoped_steps = json.loads(scoped.stdout)
            self.assertTrue(any(step['name'].endswith('-bootstrap') for step in scoped_steps))
            for step in scoped_steps:
                self.assertEqual(step['environment']['CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT'],
                                 str((Path(directory) / 'explicit-typescript').resolve()))
                self.assertEqual(step['environment']['DOTNET_CLI_USE_MSBUILD_SERVER'], '0')
                self.assertEqual(step['environment']['CARGO_CACHE_AUTO_CLEAN_FREQUENCY'], 'never')
            scoped_env = os.environ | {"CODECALLNAVIGATOR_TS_NODE_MODULES": str(Path(directory) / "custom-modules")}
            scoped_env.pop('CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT', None)
            derived = subprocess.run(
                [sys.executable, str(ROOT / "tools/verify.py"), "engine", "--suite", "integration", "--dry-run"],
                cwd=directory, env=scoped_env, capture_output=True, text=True, check=True)
            for step in json.loads(derived.stdout):
                self.assertEqual(step['environment']['CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT'],
                                 str((Path(directory) / 'custom-modules/typescript').resolve()))
            output = Path(directory) / "must-not-be-created"
            result = subprocess.run([sys.executable, str(ROOT / "tools/verify.py"), "full", "--dry-run", "--output", str(output),
                "--profile-source", "./saved-data", "--codescene-command", "./bin/cs-mcp"], cwd=directory,
                capture_output=True, text=True, check=True)
            steps = json.loads(result.stdout)
            self.assertEqual(steps[0]["name"], "prerequisites")
            commands = {step["name"]: step["command"] for step in steps}
            source = str((Path(directory) / "saved-data").resolve())
            codescene = str((Path(directory) / "bin/cs-mcp").resolve())
            preflight = commands["prerequisites"]
            self.assertEqual(preflight[preflight.index("--profile-source") + 1], source)
            self.assertEqual(commands["performance-profile-snapshot"][-2], source)
            self.assertEqual(preflight[preflight.index("--codescene-command") + 1], codescene)
            quality_command = commands["codescene"]
            self.assertEqual(quality_command[quality_command.index("--command") + 1], codescene)
            self.assertEqual(steps[-1]["name"], "cleanup")
            self.assertTrue(steps[-1]["always"])
            self.assertNotIn("--keep-artifacts", commands["cleanup"])
            for step in steps:
                self.assertEqual(step["environment"]["CARGO_INCREMENTAL"], "0")
                self.assertEqual(step["environment"]["CARGO_CACHE_AUTO_CLEAN_FREQUENCY"], "never")
            retained = subprocess.run(
                [sys.executable, str(ROOT / "tools/verify.py"), "full", "--dry-run", "--keep-artifacts", "--output", str(output)],
                cwd=directory, capture_output=True, text=True, check=True,
            )
            self.assertIn("--keep-artifacts", json.loads(retained.stdout)[-1]["command"])
            self.assertFalse(output.exists())

    def test_invalid_base_cli_reports_unavailable_without_traceback_or_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "unused-evidence"
            result = subprocess.run(
                [sys.executable, str(ROOT / "tools/verify.py"), "light", "--dry-run",
                 "--base=invalid^{not-a-revision-type}", "--output", str(output)],
                cwd=directory, capture_output=True, text=True, timeout=10,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("Verification unavailable:", result.stderr)
            self.assertNotIn("Traceback", result.stderr)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
