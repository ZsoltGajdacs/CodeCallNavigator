"""Exercise pinned dependency checkout against real local Git repositories."""

import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


BOOTSTRAP_PATH = Path(__file__).resolve().parents[1] / "tools" / "bootstrap.py"
SPEC = importlib.util.spec_from_file_location("codecallnavigator_bootstrap", BOOTSTRAP_PATH)
bootstrap = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bootstrap)


def git(repository, *arguments):
    result = subprocess.run(["git", "-C", str(repository), *arguments], check=True,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return result.stdout.strip()


class RepositoryBootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codecallnavigator-bootstrap-test-")
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.original_root = bootstrap.ROOT
        bootstrap.ROOT = root / "workspace"
        self.addCleanup(setattr, bootstrap, "ROOT", self.original_root)
        (bootstrap.ROOT / "vendor").mkdir(parents=True)
        self.upstream = root / "upstream"
        self.upstream.mkdir()
        git(self.upstream, "init", "--initial-branch=main")
        self.pinned = self.commit("pinned contents\n", "Pinned dependency")
        self.newer = self.commit("upstream changed\n", "New upstream HEAD")
        self.spec = {"repository": str(self.upstream), "revision": self.pinned}
        self.checkout = bootstrap.ROOT / "vendor" / "fixture"

    def commit(self, content, message):
        (self.upstream / "dependency.txt").write_text(content)
        git(self.upstream, "add", "dependency.txt")
        git(self.upstream, "-c", "user.name=CodeCallNavigator test", "-c",
            "user.email=codecallnavigator-test@example.invalid", "commit", "-m", message)
        return git(self.upstream, "rev-parse", "HEAD")

    def test_fresh_clone_checks_out_pin_when_upstream_has_advanced(self):
        self.assertNotEqual(self.pinned, self.newer)

        bootstrap.fetch_repository("fixture", self.spec)

        self.assertEqual(git(self.checkout, "rev-parse", "HEAD"), self.pinned)
        self.assertEqual((self.checkout / "dependency.txt").read_text(), "pinned contents\n")
        self.assertEqual(git(self.checkout, "status", "--porcelain"), "")

    def test_dirty_existing_checkout_is_preserved_when_pin_differs(self):
        subprocess.run(["git", "clone", str(self.upstream), str(self.checkout)],
                       check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        edited = self.checkout / "dependency.txt"
        edited.write_text("uncommitted user change\n")

        with self.assertRaisesRegex(RuntimeError, "Refusing to overwrite changes"):
            bootstrap.fetch_repository("fixture", self.spec)

        self.assertEqual(git(self.checkout, "rev-parse", "HEAD"), self.newer)
        self.assertEqual(edited.read_text(), "uncommitted user change\n")
        self.assertIn("dependency.txt", git(self.checkout, "status", "--porcelain"))


@unittest.skipUnless(shutil.which("dotnet"), "Requires installed .NET SDK")
class CallAtlasBuildTests(unittest.TestCase):
    def test_patched_framework_builds_cli_and_core_into_packaged_output(self):
        with tempfile.TemporaryDirectory(prefix="codecallnavigator-dotnet-build-") as temporary:
            root = Path(temporary)
            project = root / "vendor/callatlas/src/CallAtlas.Cli"
            project.mkdir(parents=True)
            source = project / "CallAtlas.Cli.csproj"
            contents = '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net11.0</TargetFramework><AssemblyName>callatlas</AssemblyName></PropertyGroup><ItemGroup><ProjectReference Include="../CallAtlas.Core/CallAtlas.Core.csproj" /></ItemGroup></Project>'
            source.write_text(contents)
            core = project.with_name("CallAtlas.Core")
            core.mkdir()
            core_source = core / "CallAtlas.Core.csproj"
            core_contents = '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>'
            core_source.write_text(core_contents)
            (core / "FixtureCore.cs").write_text('public static class FixtureCore { public static string Value => "fixture"; }')
            (project / "Program.cs").write_text('System.Console.WriteLine(FixtureCore.Value);')
            (root / "NuGet.config").write_text('<configuration><packageSources><clear /></packageSources></configuration>')
            original_root = bootstrap.ROOT
            bootstrap.ROOT = root
            try:
                bootstrap.build_callatlas()
            finally:
                bootstrap.ROOT = original_root
            output = bootstrap.callatlas_output(root)
            self.assertTrue((output / "callatlas.dll").is_file())
            runtime = json.loads((output / "callatlas.runtimeconfig.json").read_text())
            self.assertEqual(runtime["runtimeOptions"]["tfm"], "net11.0")
            self.assertEqual(source.read_text(), contents)
            self.assertEqual(core_source.read_text(), core_contents)
            self.assertTrue((core / "bin/Release/net11.0/CallAtlas.Core.dll").is_file())


class PatchBootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codecallnavigator-patch-test-")
        self.addCleanup(self.temporary.cleanup)
        self.original_root = bootstrap.ROOT
        bootstrap.ROOT = Path(self.temporary.name)
        self.addCleanup(setattr, bootstrap, "ROOT", self.original_root)
        self.repository = bootstrap.ROOT / "vendor/callatlas"
        self.repository.mkdir(parents=True)
        git(self.repository, "init", "--initial-branch=main")
        self.source = self.repository / "dependency.txt"
        self.source.write_text("upstream\n")
        (self.repository / "other.txt").write_text("unchanged\n")
        git(self.repository, "add", ".")
        git(self.repository, "-c", "user.name=CodeCallNavigator test", "-c",
            "user.email=codecallnavigator-test@example.invalid", "commit", "-m", "Pinned")
        self.spec = {}
        self.save_patch("legacy_", "assembly identities\n")
        self.save_patch("", "assembly identities\nentry metadata\n")
        self.source.write_text("upstream\n")

    def save_patch(self, prefix, content):
        self.source.write_text(content)
        difference = subprocess.check_output(["git", "-C", str(self.repository), "diff", "--binary", "HEAD"])
        name = f"{prefix}identity.patch"
        (bootstrap.ROOT / name).write_bytes(difference)
        self.spec[f"{prefix}patch"] = name
        self.spec[f"{prefix}patch_sha256"] = hashlib.sha256(difference).hexdigest()

    def assert_current(self):
        self.assertEqual(self.source.read_text(), "assembly identities\nentry metadata\n")

    def test_fresh_checkout_applies_current_patch(self):
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_current_patch_is_idempotent(self):
        self.source.write_text("assembly identities\nentry metadata\n")
        bootstrap.apply_patch(self.spec)
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_rune_patch_applies_idempotently_and_refuses_other_edits(self):
        rune = self.repository.with_name("rune")
        self.repository.rename(rune)
        bootstrap.apply_patch(self.spec, "rune")
        bootstrap.apply_patch(self.spec, "rune")
        source = rune / "dependency.txt"
        self.assertEqual(source.read_text(), "assembly identities\nentry metadata\n")
        source.write_text("user modification\n")
        with self.assertRaisesRegex(RuntimeError, "differs from the supported"):
            bootstrap.apply_patch(self.spec, "rune")
        self.assertEqual(source.read_text(), "user modification\n")

    def test_exact_legacy_patch_upgrades_without_resetting_checkout(self):
        self.source.write_text("assembly identities\n")
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_exact_previous_patch_upgrades_without_losing_legacy_support(self):
        self.save_patch("previous_", "assembly identities\nprevious entry metadata\n")
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_exact_older_patch_upgrades_without_losing_other_supported_versions(self):
        self.save_patch("older_", "assembly identities\nolder entry metadata\n")
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_exact_pre_sdk_patch_upgrades_without_losing_other_supported_versions(self):
        self.save_patch("pre_sdk_", "assembly identities\nprevious SDK target\n")
        bootstrap.apply_patch(self.spec)
        self.assert_current()

    def test_unrelated_tracked_change_is_preserved_and_refused(self):
        self.source.write_text("assembly identities\n")
        other = self.repository / "other.txt"
        other.write_text("user modification\n")
        with self.assertRaisesRegex(RuntimeError, "differs from the supported"):
            bootstrap.apply_patch(self.spec)
        self.assertEqual(other.read_text(), "user modification\n")
        self.assertEqual(self.source.read_text(), "assembly identities\n")

    def test_unrelated_untracked_source_is_preserved_and_refused(self):
        untracked = self.repository / "user.txt"
        untracked.write_text("user content\n")
        with self.assertRaisesRegex(RuntimeError, "unrelated untracked"):
            bootstrap.apply_patch(self.spec)
        self.assertEqual(untracked.read_text(), "user content\n")
        self.assertEqual(self.source.read_text(), "upstream\n")

    def test_failed_upgrade_restores_the_legacy_patch(self):
        self.source.write_text("assembly identities\n")
        patch = bootstrap.ROOT / self.spec["patch"]
        patch.write_text("invalid patch\n")
        self.spec["patch_sha256"] = hashlib.sha256(patch.read_bytes()).hexdigest()
        with self.assertRaises(subprocess.CalledProcessError):
            bootstrap.apply_patch(self.spec)
        self.assertEqual(self.source.read_text(), "assembly identities\n")

    def test_bad_patch_checksum_is_refused(self):
        (bootstrap.ROOT / self.spec["patch"]).write_text("tampered")
        with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
            bootstrap.apply_patch(self.spec)
        self.assertEqual(self.source.read_text(), "upstream\n")


if __name__ == "__main__":
    unittest.main()
