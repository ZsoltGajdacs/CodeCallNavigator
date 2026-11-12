"""The terminal installer publishes a complete package without losing an existing install."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import install_cli


class TerminalInstallTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix='terminal-install-test-')
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name).resolve()
        self.source = self.root / 'downloaded package'
        self.prefix = self.root / 'custom prefix'
        resources = self.source / 'Resources/analyzers'
        resources.mkdir(parents=True)
        (resources / 'marker').write_text('matching resources')
        (self.source / 'bin').mkdir()
        self.write_command('original')
        internal = self.source / 'bin/codecallnavigator-engine'
        internal.write_text('internal executable')
        internal.chmod(0o755)

    def write_command(self, marker, version=1):
        capabilities = json.dumps({'query_version': version, 'generation_binding': True})
        executable = self.source / 'bin/ccnavigator'
        executable.write_text(
            f'#!{sys.executable}\n'
            'from pathlib import Path\n'
            'import sys\n'
            f'CAPABILITIES = {capabilities!r}\n'
            f'MARKER = {marker!r}\n'
            'if sys.argv[1:] == ["capabilities"]:\n'
            '    print(CAPABILITIES)\n'
            'else:\n'
            '    resources = Path(__file__).resolve().parent.parent / "Resources/analyzers/marker"\n'
            '    print(MARKER + ":" + resources.read_text())\n')
        executable.chmod(0o755)

    def command(self, prefix=None):
        return (prefix or self.prefix) / 'bin/ccnavigator'

    def distribution(self, prefix=None):
        return (prefix or self.prefix) / 'share/CodeCallNavigatorEngine'

    def invoke(self, executable):
        return subprocess.check_output([str(executable), 'marker'], cwd=self.root, text=True).strip()

    def test_complete_copy_survives_download_removal_and_runs_from_path(self):
        installed = install_cli.install(self.source, self.prefix)
        self.assertEqual(installed, self.command())
        self.assertTrue(installed.is_symlink())
        self.assertEqual(installed.resolve(), self.distribution() / 'bin/ccnavigator')
        self.assertEqual((self.distribution() / 'bin/codecallnavigator-engine').read_text(),
                         'internal executable')
        shutil.rmtree(self.source)
        environment = os.environ | {'PATH': str(self.prefix / 'bin')}
        result = subprocess.check_output(['ccnavigator', 'marker'], cwd=self.root,
                                         env=environment, text=True)
        self.assertEqual(result.strip(), 'original:matching resources')

    def test_cli_default_prefix_uses_home_without_editing_shell_configuration(self):
        shutil.copy2(install_cli.__file__, self.source / 'install_cli.py')
        home = self.root / 'home'
        home.mkdir()
        shell = home / '.zshrc'
        shell.write_text('# existing configuration\n')
        result = subprocess.run([sys.executable, str(self.source / 'install_cli.py')],
                                env=os.environ | {'HOME': str(home), 'PATH': '/usr/bin:/bin'},
                                capture_output=True, text=True, check=True)
        self.assertEqual(self.invoke(home / '.local/bin/ccnavigator'),
                         'original:matching resources')
        self.assertIn('PATH', result.stdout)
        self.assertEqual(shell.read_text(), '# existing configuration\n')

    def test_cli_custom_prefix_accepts_spaces(self):
        shutil.copy2(install_cli.__file__, self.source / 'install_cli.py')
        subprocess.run([sys.executable, str(self.source / 'install_cli.py'),
                        '--prefix', str(self.prefix)], check=True, capture_output=True)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def test_reinstallation_updates_command_and_resources(self):
        install_cli.install(self.source, self.prefix)
        self.write_command('updated')
        (self.source / 'Resources/analyzers/marker').write_text('updated resources')
        install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'updated:updated resources')

    def test_installed_package_can_reinstall_itself(self):
        install_cli.install(self.source, self.prefix)
        install_cli.install(self.distribution(), self.prefix)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def test_foreign_command_file_directory_and_symlink_are_preserved(self):
        foreign = self.root / 'foreign'
        foreign.write_text('foreign command')
        for kind in ('file', 'directory', 'symlink'):
            with self.subTest(kind=kind):
                prefix = self.root / kind
                command = self.command(prefix)
                command.parent.mkdir(parents=True)
                if kind == 'directory':
                    command.mkdir()
                    (command / 'owned').write_text('foreign directory')
                elif kind == 'symlink':
                    command.symlink_to(foreign)
                else:
                    command.write_text('foreign command')
                with self.assertRaises((RuntimeError, OSError, ValueError)):
                    install_cli.install(self.source, prefix)
                self.assertFalse(self.distribution(prefix).exists())
                if kind == 'directory':
                    self.assertEqual((command / 'owned').read_text(), 'foreign directory')
                else:
                    self.assertEqual(command.read_text(), 'foreign command')
                if kind == 'symlink':
                    self.assertEqual(command.readlink(), foreign)

    def test_unowned_distribution_is_preserved(self):
        distribution = self.distribution()
        distribution.mkdir(parents=True)
        (distribution / 'owned').write_text('unrelated distribution')
        with self.assertRaises((RuntimeError, OSError, ValueError)):
            install_cli.install(self.source, self.prefix)
        self.assertEqual((distribution / 'owned').read_text(), 'unrelated distribution')
        self.assertFalse(self.command().exists())

    def test_incompatible_capabilities_preserve_previous_installation(self):
        install_cli.install(self.source, self.prefix)
        self.write_command('incompatible', version=99)
        with self.assertRaises((RuntimeError, OSError, ValueError)):
            install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def test_missing_resources_preserve_previous_installation(self):
        install_cli.install(self.source, self.prefix)
        shutil.rmtree(self.source / 'Resources/analyzers')
        with self.assertRaises((RuntimeError, OSError, ValueError)):
            install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def test_source_resource_and_ownership_symlinks_are_rejected_without_writing_target(self):
        target = self.root / 'external target'
        target.write_text('external contents')
        for relative in ('Resources/analyzers/marker', '.ccnavigator-install'):
            with self.subTest(relative=relative):
                link = self.source / relative
                original = link.read_bytes() if link.exists() else None
                link.unlink(missing_ok=True)
                link.symlink_to(target)
                try:
                    with self.assertRaisesRegex(RuntimeError, 'symlink'):
                        install_cli.install(self.source, self.prefix)
                    self.assertEqual(target.read_text(), 'external contents')
                    self.assertFalse(self.command().exists())
                    self.assertFalse(self.distribution().exists())
                finally:
                    link.unlink()
                    if original is not None:
                        link.write_bytes(original)

    def test_symlinked_destination_is_rejected_even_when_target_is_owned(self):
        install_cli.install(self.source, self.prefix)
        external = self.root / 'relocated installation'
        self.distribution().rename(external)
        self.distribution().symlink_to(external, target_is_directory=True)
        with self.assertRaisesRegex(RuntimeError, 'unrelated installation'):
            install_cli.install(self.source, self.prefix)
        self.assertEqual(self.distribution().readlink(), external)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def fail_link_publication(self):
        replace = os.replace
        failed = False

        def publish(source, destination):
            nonlocal failed
            if Path(destination) == self.command() and not failed:
                failed = True
                raise OSError('injected link publication failure')
            return replace(source, destination)

        return patch.object(install_cli.os, 'replace', side_effect=publish)

    def test_failed_update_restores_previous_distribution_and_command(self):
        install_cli.install(self.source, self.prefix)
        self.write_command('updated')
        with self.fail_link_publication():
            with self.assertRaises(OSError):
                install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')
        install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'updated:matching resources')

    def test_failed_first_publication_removes_unreachable_distribution(self):
        with self.fail_link_publication():
            with self.assertRaises(OSError):
                install_cli.install(self.source, self.prefix)
        self.assertFalse(self.command().exists())
        self.assertFalse(self.distribution().exists())
        install_cli.install(self.source, self.prefix)
        self.assertEqual(self.invoke(self.command()), 'original:matching resources')

    def test_failed_rollback_retains_previous_installation_and_reports_recovery_path(self):
        install_cli.install(self.source, self.prefix)
        self.write_command('updated')
        replace = os.replace

        def publish(source, destination):
            if Path(destination) == self.command() or Path(source).name == 'previous':
                raise OSError('injected publication and rollback failure')
            return replace(source, destination)

        with patch.object(install_cli.os, 'replace', side_effect=publish):
            with self.assertRaisesRegex(RuntimeError, 'previous installation preserved') as failure:
                install_cli.install(self.source, self.prefix)
        previous = list((self.prefix / 'share').glob('.ccnavigator-stage-*/previous'))
        self.assertEqual(len(previous), 1)
        self.assertIn(str(previous[0]), str(failure.exception))
        self.assertEqual(self.invoke(previous[0] / 'bin/ccnavigator'),
                         'original:matching resources')
        self.assertTrue(self.command().is_symlink())


if __name__ == '__main__':
    unittest.main()
