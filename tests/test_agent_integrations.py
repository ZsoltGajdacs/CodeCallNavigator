"""Agent archives must preserve canonical instructions and contain only reviewed files."""
import json
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import package_agent_integrations as package

ROOT = Path(__file__).resolve().parents[1]


class AgentPackageTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix='agent-package-test-')
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        shutil.copytree(ROOT / 'integrations/shared', self.root / 'integrations/shared')
        for host in package.HOSTS:
            shutil.copytree(ROOT / 'integrations' / host, self.root / 'integrations' / host)
        shutil.copyfile(ROOT / 'LICENSE', self.root / 'LICENSE')
        self.source = self.root / 'integrations/claude-code'
        self.skill = self.source / 'skills/graph-context/SKILL.md'

    def build(self, name):
        destination = self.root / name
        package.build(self.root, ['claude-code'], destination)
        return destination

    def test_self_contained_archive_preserves_exact_body_and_is_deterministic(self):
        outputs = [self.root / name for name in ['first', 'second']]
        for output in outputs:
            package.build(self.root, list(package.HOSTS), output)
        for host in package.HOSTS:
            with self.subTest(host=host):
                self.check_archive(host, outputs)

    def check_archive(self, host, outputs):
        name = f'codecallnavigator-{host}'
        archive = outputs[0] / f'{name}.zip'
        self.assertEqual(archive.read_bytes(), (outputs[1] / archive.name).read_bytes())
        extracted = self.root / f'extracted-{host}'
        with zipfile.ZipFile(archive) as contents:
            contents.extractall(extracted)
        plugin = extracted / name
        self.assertEqual((plugin / 'skills/graph-context/SKILL.md').read_bytes(),
                         (self.root / 'integrations' / host / package.SKILL).read_bytes())
        skill = (plugin / 'skills/graph-context/SKILL.md').read_text()
        self.assertIn('using configured CodeCallNavigator MCP tools', skill)
        self.assertIn('no silent terminal-CLI graph fallback', skill)
        self.assertNotIn('ccnavigator query', skill)
        self.assertEqual((plugin / 'LICENSE').read_bytes(), (self.root / 'LICENSE').read_bytes())
        catalog_path = package.METADATA[host][1]
        catalog = json.loads((plugin / catalog_path).read_text())
        source = catalog['plugins'][0]['source']
        self.assertEqual(source, './' if host == 'claude-code' else {'source': 'local', 'path': './'})
        self.assertFalse(any(path.is_symlink() for path in plugin.rglob('*')))

    def test_drift_fails_read_only_check_and_build_then_explicit_sync_repairs(self):
        for host in package.HOSTS:
            with self.subTest(host=host):
                path = self.root / 'integrations' / host / package.SKILL
                path.write_text(path.read_text() + '\nUnreviewed instruction.\n')
                before = path.read_bytes()
                with self.assertRaisesRegex(RuntimeError, 'drift'):
                    package.validate(self.root, host)
                self.assertEqual(path.read_bytes(), before)
                with self.assertRaisesRegex(RuntimeError, 'drift'):
                    package.build(self.root, list(package.HOSTS), self.root / 'refused')
                self.assertFalse((self.root / 'refused').exists())
                package.synchronize(self.root, host)
                package.validate(self.root, host)
                self.assertNotEqual(path.read_bytes(), before)

    def test_unexpected_hook_file_is_rejected_before_output_is_created(self):
        (self.source / 'hooks').mkdir()
        (self.source / 'hooks/hooks.json').write_text('{}')
        with self.assertRaisesRegex(RuntimeError, 'Unexpected'):
            self.build('refused')
        self.assertFalse((self.root / 'refused').exists())

    def test_symlinked_file_and_directory_are_rejected(self):
        for location in ['README.md', 'skills']:
            with self.subTest(location=location):
                target = self.source / location
                saved = self.root / ('saved-' + target.name)
                target.rename(saved)
                target.symlink_to(saved, target_is_directory=saved.is_dir())
                with self.assertRaisesRegex(RuntimeError, 'symlink'):
                    self.build('refused')
                self.assertFalse((self.root / 'refused').exists())
                target.unlink()
                saved.rename(target)

    def test_catalog_escape_is_rejected_before_publication(self):
        path = self.source / '.claude-plugin/marketplace.json'
        catalog = json.loads(path.read_text())
        catalog['plugins'][0]['source'] = '../outside'
        path.write_text(json.dumps(catalog))
        with self.assertRaisesRegex(RuntimeError, 'catalog'):
            self.build('refused')
        self.assertFalse((self.root / 'refused').exists())

    def test_existing_output_is_never_overwritten(self):
        output = self.build('first')
        archive = output / 'codecallnavigator-claude-code.zip'
        before = archive.read_bytes()
        with self.assertRaisesRegex(RuntimeError, 'exists'):
            package.build(self.root, ['claude-code'], output)
        self.assertEqual(archive.read_bytes(), before)

    def test_failed_assembly_does_not_publish_a_partial_output(self):
        output = self.root / 'retryable'
        with patch.object(package, 'write_archive', side_effect=OSError('disk failure')):
            with self.assertRaisesRegex(OSError, 'disk failure'):
                package.build(self.root, ['claude-code'], output)
        self.assertFalse(output.exists())
        self.assertFalse(list(self.root.glob('.agent-integrations-*')))
        package.build(self.root, ['claude-code'], output)
        self.assertTrue((output / 'codecallnavigator-claude-code.zip').is_file())

    def test_mcp_manifest_extension_and_canonical_symlink_are_rejected(self):
        manifest = self.source / '.claude-plugin/plugin.json'
        original = manifest.read_bytes()
        value = json.loads(original)
        value['mcpServers'] = './.mcp.json'
        manifest.write_text(json.dumps(value))
        with self.assertRaisesRegex(RuntimeError, 'manifest'):
            self.build('refused')
        self.assertFalse((self.root / 'refused').exists())
        manifest.write_bytes(original)
        canonical = self.root / 'integrations/shared/graph-context.md'
        saved = canonical.with_suffix('.saved')
        canonical.rename(saved)
        canonical.symlink_to(saved)
        with self.assertRaisesRegex(RuntimeError, 'symlink'):
            self.build('refused')
        self.assertFalse((self.root / 'refused').exists())

    def test_codex_manifest_rejects_hooks_and_requires_a_boolean_local_executor(self):
        path = self.root / 'integrations/codex/plugin.json'
        original = json.loads(path.read_text())
        for setting in [{'requires_local_executor': 1},
                        {'requires_local_executor': True, 'hooks': './hooks.json'}]:
            with self.subTest(setting=setting):
                value = original | {'extensions': {'com.openai': setting}}
                path.write_text(json.dumps(value))
                with self.assertRaisesRegex(RuntimeError, 'local executor'):
                    package.validate(self.root, 'codex')

    def test_codex_catalog_escape_and_install_policy_changes_are_rejected(self):
        path = self.root / 'integrations/codex/.agents/plugins/marketplace.json'
        original = path.read_text()
        for field, setting in [('source', {'source': 'local', 'path': '../outside'}),
                               ('policy', {'installation': 'INSTALLED_BY_DEFAULT',
                                           'authentication': 'ON_USE'})]:
            with self.subTest(field=field):
                value = json.loads(original)
                value['plugins'][0][field] = setting
                path.write_text(json.dumps(value))
                with self.assertRaisesRegex(RuntimeError, 'catalog'):
                    package.build(self.root, list(package.HOSTS), self.root / 'refused')
                self.assertFalse((self.root / 'refused').exists())

    def test_malformed_manifest_shapes_and_versions_fail_cleanly_for_both_hosts(self):
        for host in package.HOSTS:
            path = self.root / 'integrations' / host / package.METADATA[host][0]
            original = path.read_text()
            for value in [None, [], 'not-an-object', 3]:
                with self.subTest(host=host, value=value):
                    path.write_text(json.dumps(value))
                    with self.assertRaisesRegex(RuntimeError, 'JSON object'):
                        package.validate(self.root, host)
            for version in [None, 1, True, [], {}, 'not-a-version']:
                with self.subTest(host=host, version=version):
                    value = json.loads(original)
                    value['version'] = version
                    path.write_text(json.dumps(value))
                    with self.assertRaisesRegex(RuntimeError, 'version'):
                        package.validate(self.root, host)
            path.write_text(original)


if __name__ == '__main__':
    unittest.main()
