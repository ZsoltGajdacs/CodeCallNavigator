---
id: doc-23
title: Release maintenance and publication runbook
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:16:00Z'
---
# Release maintenance and publication runbook

Migrated 2026-11-16 from `docs/public-releases.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Automatic GitHub releases

The [macOS release workflow](<../../../.github/workflows/release.yml>) runs when a new tag
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
public-preview image), Python 3.13, .NET 11, JDK 25, Rust 1.99.1 and Node 24+.
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
