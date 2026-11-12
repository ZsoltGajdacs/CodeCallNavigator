# Install the terminal command

Extract the standalone `CodeCallNavigatorEngine` distribution, then run its
installer with Python 3.12+:

```sh
python3.13 /path/to/CodeCallNavigatorEngine/install.py
ccnavigator capabilities
ccnavigator --help
```

For a local build, first run `python3.13 tools/package.py --engine-only`, then
`python3.13 dist/CodeCallNavigatorEngine/install.py`.

The installer copies the complete distribution, including analyzer resources,
to `~/.local/share/CodeCallNavigatorEngine` and links `~/.local/bin/ccnavigator`
to its public executable. You can then delete the extracted download. The
language toolchains required for analysis must still be installed.

If `~/.local/bin` is absent from your PATH, the installer prints the exact
`export PATH=...` command to add to your shell configuration. It does not edit
shell configuration itself. Open a new terminal after updating that configuration.

## Custom prefix and updates

```sh
python3.13 /path/to/CodeCallNavigatorEngine/install.py --prefix /path/to/prefix
```

This installs the distribution under `PREFIX/share/CodeCallNavigatorEngine` and
the command under `PREFIX/bin`. Run without `sudo` for a user-owned prefix.

To update, run the new download's installer with the same prefix. It stages and
checks the new executable before replacing an installer-owned distribution.
If publication fails, it restores the previous installation. Unrelated commands
or distribution directories cause installation to stop without overwriting them.
Packages containing symlinks are rejected so the installed files cannot depend
on the extracted download. Official standalone distributions contain regular files.

Package names and the internal `codecallnavigator-engine` executable remain
unchanged. Existing desktop and MCP configurations continue to use that executable.
The terminal command exposes the same arguments, JSON contracts and exit codes.

## Removal

For the default prefix, remove the installer-created link and distribution:

```sh
rm "$HOME/.local/bin/ccnavigator"
rm -r "$HOME/.local/share/CodeCallNavigatorEngine"
```

For a custom prefix, use its corresponding paths. Project graphs, caches and
desktop application data are stored separately and are preserved.
