"""Exercise pinned dependency checkout against real local Git repositories."""

import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest


BOOTSTRAP_PATH = Path(__file__).resolve().parents[1] / "tools" / "bootstrap.py"
SPEC = importlib.util.spec_from_file_location("codegraph_bootstrap", BOOTSTRAP_PATH)
bootstrap = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(bootstrap)


def git(repository, *arguments):
    result = subprocess.run(["git", "-C", str(repository), *arguments], check=True,
                            text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return result.stdout.strip()


class RepositoryBootstrapTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="codegraph-bootstrap-test-")
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
        git(self.upstream, "-c", "user.name=CodeGraph test", "-c",
            "user.email=codegraph-test@example.invalid", "commit", "-m", message)
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


if __name__ == "__main__":
    unittest.main()
