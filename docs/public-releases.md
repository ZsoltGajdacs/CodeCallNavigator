# Public releases and corresponding source

CodeCallNavigator is copyright (C) 2026 Zsolt Gajdacs and licensed under
AGPL-3.0-only. Dependencies retain their own licenses; see
[THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). The license permits commercial
use and independent forks. Analyzing a repository does not change that
repository's license. There is no warranty or guaranteed support.

The current build targets Apple Silicon macOS. Linux downloads are a future
possibility, not a supported platform today.

Download matching binary and source assets from
[CodeCallNavigator releases](https://github.com/ZsoltGajdacs/CodeCallNavigator/releases).
Select the same release for both; the source asset is
`CodeCallNavigator-source.tar.gz`. A local development build may have no published
release; its source identity records the local commit and changes.

## Official release assets

Official releases publish the app ZIP, standalone engine ZIP, matching
`CodeCallNavigator-source.tar.gz`, and `SHA256SUMS` together. The macOS app is ad hoc
signed; notarization and Linux packages are not provided. Download matching source
and binaries from the same release and verify their checksums.

## Corresponding source and redistribution

Public binary downloads must include their exact corresponding source and notices.
Keep the source available without charge for as long as the binary download is
offered. GitHub's automatic source ZIP omits the dependency sources and is not a
substitute for `CodeCallNavigator-source.tar.gz`. Do not redistribute an older
binary beside a newer source archive.

`SOURCE_IDENTITY.json` inside the app identifies project inputs. The external
`CodeCallNavigator.app.build.json` and engine `BUILD_SOURCE.json` bind source identity
to packaged file hashes. The source archive preserves those manifests;
`SHA256SUMS` binds the downloadable ZIPs and source archive together. Normal local
`python3.13 tools/package.py` builds do not create a complete public source offer.

## Rebuild from the source archive

Extract `CodeCallNavigator-source.tar.gz`. Verify each file against
`SOURCE_MANIFEST.json` before rebuilding; it includes the project commit and
SHA256s. The archive contains:

- `project.tar`: the committed project tree, including adapters, build scripts,
  lockfiles, notices and the exact checked-in upstream patches.
- `project.bundle`: the project's Git history at the release commit.
- `upstreams/*.bundle`: the pinned Rune, CallAtlas and java-callgraph2 Git
  histories. These preserve the identities used by bootstrap verification.
- `cargo-vendor/` and `cargo-config.toml`: Cargo registry sources and a config
  pointing to their sibling directory.
- `dependency-sources/`: Graphviz and libtool source archives for desktop
  releases, the full Aviator source archive, and native build formula/receipt
  information. These separate programs/libraries keep their original licenses.
- `builds/`: packaged file inventories and source identities.

From the extracted `CodeCallNavigator-source` directory:

```sh
git clone project.bundle project
cd project
mkdir -p vendor .cargo
git clone ../upstreams/rune.bundle vendor/rune
git clone ../upstreams/callatlas.bundle vendor/callatlas
git clone ../upstreams/java-callgraph2.bundle vendor/java-callgraph2
cp ../cargo-config.toml .cargo/config.toml
python3.13 tools/bootstrap.py
python3.13 tools/package.py
```

Bootstrap checks out the locked revisions and applies the recorded patches;
no upstream contribution is required to reproduce those changes. The archived
`project.tar` is also directly readable, but use the bundle clone for the existing
Git-based bootstrap and provenance checks. Cargo, NuGet and Gradle build tools
remain prerequisites. NuGet/Gradle may restore permissively licensed dependencies
from their registries; this is not an entirely offline SDK distribution or a
promise of bit-for-bit reproducibility.

To rebuild the separate native components, extract the Graphviz/libtool archives
and use their included build instructions and the archived Homebrew formulae.
`NATIVE_SOURCES.json` records the actual installed versions and build receipts.
The packager relocates dynamic libraries and signs them ad hoc. LGPL libraries
remain separate and replaceable; users may modify them and reverse engineer the
application to debug those modifications. Re-sign the modified app ad hoc when
needed. Aviator's complete source archive includes its Maven build files.

## License review boundary

AGPL covers project-owned desktop, engine and adapter code. Rune and CallAtlas
are MIT. The Java bytecode helper is project-owned AGPL code; its dependencies
retain Apache/MIT/LGPL terms.
Graphviz is a separate EPL-2.0 executable used through its process interface,
not a library linked into the AGPL desktop; its actual installed COPYING file
is authoritative over stale package-manager metadata. libltdl is a separate
LGPL-2.1-or-later dynamic library loaded by that executable. Neither source
archive nor project license replaces dependency terms. Rust dependency notices
are collected with a machine-readable inventory; their sources are preserved by
Cargo vendoring. JDT/Lombok investigation downloads and project-installed Node,
TypeScript, React and Next.js are not shipped in the production package.

The current reviewed native closure is Graphviz plus libltdl. Packaging fails
when that closure grows until the new dependency has source and notice coverage.
Check this document, the source lock, installed receipts, bundled notices and
actual archive contents before each public release. Publishing Linux binaries
will require its own packaging and license/source review.
