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

## Automatic GitHub releases

The [macOS release workflow](../.github/workflows/release.yml) runs when a new tag
is pushed. Git tags do not belong to branches, so the workflow checks that the
tag's commit is in `main` history before installing tools or building anything.
Tags on unmerged branches fail that check; moving or deleting a tag does not
publish another release. Both lightweight and annotated tags are supported.

For example, after updating `main` locally:

```sh
git switch main
git pull --ff-only
git tag -a v0.1.0 -m 'Release v0.1.0'
git push origin v0.1.0
```

Use a new tag name for each release. The tag must include the workflow itself.
The workflow uses GitHub's `xcode-27` Apple Silicon/macOS 27 runner (currently a
public-preview image), Python 3.13, .NET 10, JDK 25, Rust 1.99.1 and Node 24+.
Immutable, checksum-pinned official Homebrew bottles provide the reviewed
Graphviz/libtool versions; their matching sources are also verified and bundled.
Provisioning explicitly updates the runner's Homebrew CLI to at least 7.0.13,
which supports installing these local bottles without disabling path restrictions.
The native installer refuses to run on developer or self-hosted machines.

Successful tag builds publish these assets together:

- `CodeCallNavigator.zip`: the ad hoc signed macOS application.
- `CodeCallNavigatorEngine.zip`: the standalone engine and analyzer resources.
- `CodeCallNavigator-source.tar.gz`: the complete matching source described below.
- `SHA256SUMS`: checksums of all three archives.

Build jobs have read-only repository permission. A separate publishing job gets
write permission only after a successful build. It uploads all assets to a draft
before making the release public, so failed uploads leave an unpublished draft.
Reruns may resume a draft created for the same commit; unrelated drafts and
already published release assets are not overwritten. Concurrency is serialized
per tag. Nothing in this workflow adds notarization or Linux support.

To verify the hosted build without creating a tag or publishing a release, open
GitHub Actions, choose **macOS release**, and use **Run workflow** on `main`, or:

```sh
gh workflow run release.yml --ref main
```

Manual runs execute the same build and artifact checks, retaining the four files
as the `macos-release` workflow artifact for seven days. The publish job is
skipped. The first remote run and its result are recorded with the tracked task.

## Prepare a release

Normal `python3.13 tools/package.py` creates a **local build**, not a complete
public release. For public downloads, use a committed checkout and:

1. Run the prerequisites and bootstrap commands in the README. Obtain the exact
   source archives listed in `tools/release-sources.lock.json` and place them in
   `vendor/release-sources/` with their recorded filenames. The release packager
   checks SHA256s; it never treats an upstream URL alone as a source offer.
2. Run `python3.13 tools/package.py --release`. It verifies pinned upstream
   checkouts/patches, builds the app and engine, collects notices, signs the app,
   and produces `dist/CodeCallNavigator.zip`, `CodeCallNavigatorEngine.zip`,
   `CodeCallNavigator-source.tar.gz`, and `SHA256SUMS`. Use `--engine-only
   --release` for an engine-only release. Cargo dependencies must be cached:
   source capture uses `cargo vendor --locked --offline`.
3. Inspect the source archive and extract/rebuild it as described below. Inspect
   `Contents/Resources/Licenses` in the app and `Resources/Licenses` in the engine.
   Check `codesign --verify --deep --strict dist/CodeCallNavigator.app`.
4. Publish the binary ZIPs, matching source archive and `SHA256SUMS` together in
   the same GitHub release. Keep the source available without charge for as long
   as the download is offered. Link the source archive explicitly in the release
   notes; GitHub's automatic source ZIP is insufficient because it omits vendor
   dependencies. Do not publish an older DMG beside a newer source archive.

`SOURCE_IDENTITY.json` inside the app identifies its project inputs. The external
`CodeCallNavigator.app.build.json` and engine `BUILD_SOURCE.json` bind the source
identity to packaged file hashes. The source archive preserves those build
manifests. `SHA256SUMS` binds the downloadable ZIPs and source archive together.
The app's manifest is external so creating it does not invalidate code signing.

The release gate rejects dirty/hidden source changes, mismatched build manifests,
missing or corrupt required dependency sources, unreviewed native dependencies,
and native source versions outside the source lock. Review source/license changes
when updating dependencies; do not update checksums just to bypass a failure.

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
