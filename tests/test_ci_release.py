"""Exercise release eligibility and publication without publishing test releases."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / "tools/ci/release.py"
SPEC = importlib.util.spec_from_file_location("ci_release", SCRIPT)
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


def git(root, *arguments):
    return subprocess.check_output(["git", "-C", str(root), *arguments], stderr=subprocess.DEVNULL).decode().strip()


class TagEligibilityTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        git(self.root, "init", "--initial-branch=main")
        git(self.root, "config", "user.name", "Release test")
        git(self.root, "config", "user.email", "release@example.invalid")
        (self.root / "source").write_text("main source")
        git(self.root, "add", ".")
        git(self.root, "commit", "-m", "Main")
        self.main = git(self.root, "rev-parse", "HEAD")
        git(self.root, "update-ref", "refs/remotes/origin/main", self.main)

    def test_lightweight_and_annotated_tags_on_main_are_eligible(self):
        git(self.root, "tag", "v1")
        git(self.root, "tag", "-a", "v2", "-m", "Annotated release")
        for name in ("v1", "v2"):
            self.assertEqual(release.verify_main_commit(self.root, "refs/tags/" + name), self.main)

    def test_tag_on_unmerged_branch_is_rejected(self):
        git(self.root, "checkout", "-b", "feature")
        (self.root / "source").write_text("unmerged source")
        git(self.root, "commit", "-am", "Unmerged")
        git(self.root, "tag", "feature-tag")
        with self.assertRaisesRegex(RuntimeError, "main"):
            release.verify_main_commit(self.root, "refs/tags/feature-tag")

    def test_moved_tag_cannot_publish_an_older_checkout(self):
        git(self.root, "tag", "v1")
        (self.root / "source").write_text("new source")
        git(self.root, "commit", "-am", "New commit")
        git(self.root, "update-ref", "refs/remotes/origin/main", git(self.root, "rev-parse", "HEAD"))
        with self.assertRaisesRegex(RuntimeError, "checkout"):
            release.verify_main_commit(self.root, "refs/tags/v1")

    def test_manual_build_requires_main_ref(self):
        git(self.root, "checkout", "--detach")
        git(self.root, "branch", "-D", "main")
        self.assertEqual(release.verify_main_commit(self.root, "refs/heads/main"), self.main)
        with self.assertRaisesRegex(RuntimeError, "tag or main"):
            release.verify_main_commit(self.root, "refs/heads/feature")


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.output = Path(self.temporary.name)
        lines = []
        for name in release.ASSET_NAMES:
            content = ("built asset " + name).encode()
            (self.output / name).write_bytes(content)
            lines.append(f"{hashlib.sha256(content).hexdigest()}  {name}\n")
        (self.output / "SHA256SUMS").write_text("".join(lines))

    def test_corrupt_asset_cannot_publish(self):
        (self.output / release.ASSET_NAMES[0]).write_bytes(b"corrupt binary")
        with self.assertRaisesRegex(RuntimeError, "checksum"):
            release.verify_assets(self.output)

    def test_missing_archive_is_rejected_before_github_mutation(self):
        (self.output / release.ASSET_NAMES[-1]).unlink()
        with patch.object(release, "gh") as gh:
            with self.assertRaises(FileNotFoundError):
                release.publish(self.output, "v1", "owner/repo", "a" * 40)
        gh.assert_not_called()

    def test_duplicate_checksum_entry_is_rejected_before_github_mutation(self):
        manifest = self.output / "SHA256SUMS"
        lines = manifest.read_text().splitlines()
        lines[-1] = lines[0]
        manifest.write_text("\n".join(lines) + "\n")
        with patch.object(release, "gh") as gh:
            with self.assertRaisesRegex(RuntimeError, "exactly the three"):
                release.publish(self.output, "v1", "owner/repo", "a" * 40)
        gh.assert_not_called()

    def test_publication_uses_draft_until_all_assets_are_uploaded(self):
        commands = []

        def gh(*arguments, **kwargs):
            commands.append(arguments)
            if arguments[:2] == ("release", "view"):
                return None
            return ""

        with patch.object(release, "gh", side_effect=gh):
            release.publish(self.output, "v1", "owner/repo", "a" * 40)
        operations = [command[1] for command in commands]
        self.assertEqual(operations, ["view", "create", "upload", "edit"])
        self.assertIn("--draft", commands[1])
        self.assertIn("--draft=false", commands[-1])
        for name in (*release.ASSET_NAMES, "SHA256SUMS"):
            self.assertIn(self.output / name, commands[2])

    def test_failed_upload_leaves_release_unpublished(self):
        commands = []

        def gh(*arguments, **kwargs):
            commands.append(arguments)
            if arguments[:2] == ("release", "view"):
                return None
            if arguments[:2] == ("release", "upload"):
                raise RuntimeError("upload failed")
            return ""

        with patch.object(release, "gh", side_effect=gh):
            with self.assertRaisesRegex(RuntimeError, "upload failed"):
                release.publish(self.output, "v1", "owner/repo", "a" * 40)
        self.assertFalse(any(command[1] == "edit" for command in commands))

    def test_unrelated_draft_cannot_be_overwritten(self):
        draft = json.dumps({"isDraft": True, "body": "Someone else's draft"})
        with patch.object(release, "gh", return_value=draft) as gh:
            with self.assertRaisesRegex(RuntimeError, "draft"):
                release.publish(self.output, "v1", "owner/repo", "a" * 40)
        self.assertEqual(gh.call_count, 1)

    def test_owned_draft_resumes_upload_without_creating_another_release(self):
        commit = "a" * 40
        draft = json.dumps({"isDraft": True, "body": f"<!-- CodeCallNavigator release {commit} -->"})
        with patch.object(release, "gh", side_effect=[draft, "", ""]) as gh:
            release.publish(self.output, "v1", "owner/repo", commit)
        operations = [call.args[1] for call in gh.call_args_list]
        self.assertEqual(operations, ["view", "upload", "edit"])
        self.assertIn("--clobber", gh.call_args_list[1].args)
        self.assertIn("--draft=false", gh.call_args_list[-1].args)

    def test_already_published_release_does_not_overwrite_assets(self):
        published = json.dumps({"isDraft": False, "body": "Published notes"})
        with patch.object(release, "gh", return_value=published) as gh:
            release.publish(self.output, "v1", "owner/repo", "a" * 40)
        self.assertEqual(gh.call_count, 1)
        self.assertEqual(gh.call_args.args[:2], ("release", "view"))


if __name__ == "__main__":
    unittest.main()
