"""CI provisioning rejects unsafe hosts and unverified native artifacts."""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("prepare_macos", ROOT / "tools/ci/prepare_macos.py")
CI = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CI)


class MacOSProvisioningTests(unittest.TestCase):
    def test_developer_or_self_hosted_machine_is_rejected_before_mutation(self):
        for environment in ({}, {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "self-hosted"}):
            with patch.dict(os.environ, environment, clear=True), patch.object(CI.subprocess, "run") as run:
                with self.assertRaisesRegex(RuntimeError, "GitHub-hosted"):
                    CI.require_ci_runner()
                run.assert_not_called()

    def test_runner_must_match_macos27_arm64(self):
        environment = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted"}
        with patch.dict(os.environ, environment, clear=True), \
             patch.object(CI.platform, "system", return_value="Darwin"), \
             patch.object(CI.platform, "machine", return_value="arm64"), \
             patch.object(CI.platform, "mac_ver", return_value=("27.0.1", (), "")):
            CI.require_ci_runner()
        with patch.dict(os.environ, environment, clear=True), \
             patch.object(CI.platform, "system", return_value="Darwin"), \
             patch.object(CI.platform, "machine", return_value="x86_64"):
            with self.assertRaisesRegex(RuntimeError, "Apple Silicon"):
                CI.require_ci_runner()

    def test_corrupt_bottle_is_never_published_to_cache(self):
        content = b"official bottle"
        specification = {"version": "1", "filename": "dependency.bottle.tar.gz",
                         "sha256": hashlib.sha256(content).hexdigest()}
        responses = [io.BytesIO(b'{"token":"public-pull-token"}'), io.BytesIO(b"corrupt bottle")]
        with tempfile.TemporaryDirectory() as temporary, \
             patch.object(CI.urllib.request, "urlopen", side_effect=responses):
            destination = Path(temporary) / specification["filename"]
            with self.assertRaisesRegex(RuntimeError, "checksum"):
                CI.fetch_bottle("graphviz", specification, destination)
            self.assertFalse(destination.exists())

    def test_public_registry_token_is_used_only_for_bottle_download(self):
        content = b"official bottle"
        checksum = hashlib.sha256(content).hexdigest()
        specification = {"version": "1", "filename": "dependency.bottle.tar.gz", "sha256": checksum}
        responses = [io.BytesIO(b'{"token":"public-pull-token"}'), io.BytesIO(content)]
        with tempfile.TemporaryDirectory() as temporary, \
             patch.object(CI.urllib.request, "urlopen", side_effect=responses) as request:
            destination = Path(temporary) / specification["filename"]
            CI.fetch_bottle("graphviz", specification, destination)
            self.assertEqual(destination.read_bytes(), content)
            token_url = request.call_args_list[0].args[0]
            self.assertIn("repository%3Ahomebrew%2Fcore%2Fgraphviz%3Apull", token_url)
            download_request = request.call_args_list[1].args[0]
            self.assertEqual(download_request.full_url,
                             "https://ghcr.io/v2/homebrew/core/graphviz/blobs/sha256:" + checksum)
            self.assertEqual(download_request.get_header("Authorization"), "Bearer public-pull-token")

    def test_installation_replaces_only_reviewed_native_packages_in_dependency_order(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            specifications = {name: {"version": version, "filename": name + ".bottle.tar.gz"}
                              for name, version in (("libtool", "2.6.2"), ("graphviz", "16.1.0"))}
            operations = []

            def run(arguments, **kwargs):
                operations.append(arguments)
                self.assertEqual(kwargs["env"]["HOMEBREW_NO_AUTO_UPDATE"], "1")
                outputs = {"--version": "Homebrew 7.0.13\n", "libtool": "libtool 2.6.2\n"}
                output = outputs.get(arguments[-1], "graphviz 16.1.0\n")
                return subprocess.CompletedProcess(arguments, 0, stdout=output)

            with patch.object(CI, "require_ci_runner"), \
                 patch.object(CI, "verified_archive", side_effect=lambda directory, spec: directory / spec["filename"]), \
                 patch.object(CI.subprocess, "run", side_effect=run):
                CI.install_bottles(specifications, root)
            installs = [operation for operation in operations if operation[1] == "install"]
            self.assertEqual(operations[:2], [["brew", "update"], ["brew", "--version"]])
            self.assertEqual([operation[-1] for operation in installs],
                             [str(root / "libtool.bottle.tar.gz"), str(root / "graphviz.bottle.tar.gz")])
            removals = [operation[-1] for operation in operations if operation[1] == "uninstall"]
            self.assertEqual(set(removals), {"libtool", "graphviz"})

    def test_old_homebrew_is_rejected_before_removing_packages(self):
        specifications = {name: {"version": version, "filename": name + ".bottle.tar.gz"}
                          for name, version in (("libtool", "2.6.2"), ("graphviz", "16.1.0"))}
        results = [subprocess.CompletedProcess(["brew", "update"], 0, stdout=""),
                   subprocess.CompletedProcess(["brew", "--version"], 0, stdout="Homebrew 7.0.12\n")]
        with tempfile.TemporaryDirectory() as temporary, \
             patch.object(CI, "require_ci_runner"), \
             patch.object(CI, "verified_archive", side_effect=lambda directory, spec: directory / spec["filename"]), \
             patch.object(CI, "brew", side_effect=results) as brew:
            with self.assertRaisesRegex(RuntimeError, "7.0.13"):
                CI.install_bottles(specifications, Path(temporary))
            self.assertEqual([call.args for call in brew.call_args_list], [("update",), ("--version",)])

    def test_native_bottles_match_existing_source_versions(self):
        specifications = json.loads((ROOT / "tools/ci/native-bottles.lock.json").read_text())
        sources = json.loads((ROOT / "tools/release-sources.lock.json").read_text())
        self.assertEqual(set(specifications), {"libtool", "graphviz"})
        for name, specification in specifications.items():
            self.assertEqual(specification["version"], sources[name]["version"])
            self.assertEqual(len(specification["sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
