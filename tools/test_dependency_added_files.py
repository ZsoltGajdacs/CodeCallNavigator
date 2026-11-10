"""Verify complete upstream patches without changing the checkout's real index."""
import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest

import bootstrap


class AddedSourcePatch(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='codegraph-added-patch-')
        self.root = Path(self.temporary.name)
        self.repository = self.root / 'vendor/example'
        self.repository.mkdir(parents=True)
        self.git('init', '--quiet')
        self.git('config', 'core.abbrev', '8')
        self.tracked = self.repository / 'Existing.cs'
        self.added = self.repository / 'New adapter.cs'
        self.tracked.write_text('base\n')
        self.git('add', '.')
        self.git('-c', 'commit.gpgsign=false', '-c', 'user.name=Patch test',
                 '-c', 'user.email=patch-test@example.invalid', 'commit', '--quiet', '-m', 'Base')
        self.original_root = bootstrap.ROOT
        bootstrap.ROOT = self.root
        self.spec = {}
        self.record_patch('previous_', 'previous\n', False)
        self.record_patch('', 'current\n', True)
        self.clean()

    def tearDown(self):
        bootstrap.ROOT = self.original_root
        self.temporary.cleanup()

    def git(self, *arguments):
        return subprocess.check_output(['git', '-C', str(self.repository), *arguments], stderr=subprocess.STDOUT)

    def clean(self):
        self.git('reset', '--hard', '--quiet', 'HEAD')
        self.added.unlink(missing_ok=True)

    def record_patch(self, prefix, content, added):
        self.clean()
        self.tracked.write_text(content)
        if added:
            self.added.write_text('new source\n')
        self.git('add', '.')
        patch = self.git('diff', '--cached', '--binary', 'HEAD')
        name = prefix + 'source.patch'
        (self.root / name).write_bytes(patch)
        self.spec[prefix + 'patch'] = name
        self.spec[prefix + 'patch_sha256'] = hashlib.sha256(patch).hexdigest()

    def index(self):
        return (self.repository / '.git/index').read_bytes()

    def test_clean_install_and_repeated_verification_include_added_source(self):
        bootstrap.apply_patch(self.spec, 'example')
        before = self.index()
        bootstrap.apply_patch(self.spec, 'example')
        self.assertEqual(bootstrap.checkout_patch(['git', '-C', str(self.repository)]),
                         (self.root / self.spec['patch']).read_bytes())
        self.assertEqual(self.index(), before)
        self.assertEqual(self.git('diff', '--cached', 'HEAD'), b'')
        self.assertEqual(self.added.read_text(), 'new source\n')

    def test_exact_previous_patch_migrates_to_patch_with_added_source(self):
        self.git('apply', str(self.root / self.spec['previous_patch']))
        bootstrap.apply_patch(self.spec, 'example')
        self.assertEqual(self.tracked.read_text(), 'current\n')
        self.assertEqual(self.added.read_text(), 'new source\n')
        bootstrap.apply_patch(self.spec, 'example')

    def test_named_prior_patch_checkpoint_can_migrate_without_dropping_older_checkpoints(self):
        prior = self.spec.pop('previous_patch')
        prior_hash = self.spec.pop('previous_patch_sha256')
        self.spec.update(pre_framework_patch=prior, pre_framework_patch_sha256=prior_hash)
        self.git('apply', str(self.root / prior))
        bootstrap.apply_patch(self.spec, 'example')
        self.assertEqual(self.tracked.read_text(), 'current\n')
        self.assertEqual(self.added.read_text(), 'new source\n')
        bootstrap.apply_patch(self.spec, 'example')

    def test_unrelated_untracked_source_is_rejected_without_index_or_source_changes(self):
        bootstrap.apply_patch(self.spec, 'example')
        extra = self.repository / 'Unrelated.cs'
        extra.write_text('unrelated\n')
        before = self.index()
        with self.assertRaisesRegex(RuntimeError, 'Dependency source'):
            bootstrap.apply_patch(self.spec, 'example')
        self.assertEqual(self.index(), before)
        self.assertEqual(extra.read_text(), 'unrelated\n')
        self.assertEqual(self.added.read_text(), 'new source\n')
        self.assertEqual(self.tracked.read_text(), 'current\n')

    def test_unrelated_staged_addition_is_not_hidden_by_untracked_patch_files(self):
        bootstrap.apply_patch(self.spec, 'example')
        extra = self.repository / 'Staged.cs'
        extra.write_text('staged user source\n')
        self.git('add', '--', extra.name)
        before = self.index()
        with self.assertRaisesRegex(RuntimeError, 'Dependency source differs'):
            bootstrap.apply_patch(self.spec, 'example')
        self.assertEqual(self.index(), before)
        self.assertEqual(extra.read_text(), 'staged user source\n')


if __name__ == '__main__':
    unittest.main()
