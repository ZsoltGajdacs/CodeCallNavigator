# Tag-triggered macOS release workflow

Task: TASK-44. Date: 2026-11-12.

## Delivered behavior

`.github/workflows/release.yml` handles newly pushed tags, checks their peeled
commit belongs to `origin/main`, and builds Apple Silicon macOS binaries plus
matching corresponding source. Lightweight and annotated tags are supported.
Moved/deleted tags do not publish. Manual dispatch on `main` builds and uploads
artifacts without publishing a release.

The build uses read-only repository permission. Publishing is a separate job
with contents-write permission after successful build/upload. The publishing
helper verifies archive checksums, creates or resumes its own commit-marked
draft, uploads all four assets, and only then publishes. It leaves unrelated
drafts and existing published release assets unchanged.

## Local verification

- Planner, explorer, implementer, reviewer and tester roles completed.
- `actionlint .github/workflows/release.yml`: passed. The config explicitly
  recognizes GitHub's official `xcode-27` preview label, which the installed
  actionlint does not yet list by default.
- 19 CI tests passed: real Git ancestry/tag fixtures, checksum rejection,
  draft-first publication, failure/retry behavior and CI-only provisioning.
- 15 existing packaging tests and 6 source-release tests passed.
- All four modified Python source/test files score CodeScene 10.0; pre-commit
  safeguards passed before implementation and compatibility-fix commits.
- No local Homebrew installation/removal/update was performed. No app or Rust
  implementation was changed, and no unrelated runtime suite was rerun.

## Hosted integration

First run: [43166736110](https://github.com/ZsoltGajdacs/CodeCallNavigator/actions/runs/43166736110),
implementation commit `e85efd41d6edc571439f5aaf494f6d482dfc6135`.
It failed when the runner's Homebrew 7.0.12 treated the verified bottle pathname
as a formula name. Source/bottle downloads and checksums had succeeded. Upstream
[fix 31986b7](https://github.com/Homebrew/brew/commit/31986b7edd6b83cfdc0787c4a9e4d4a5aeafab46),
released in 7.0.13, restores explicit bottle dispatch without bypassing generic
formula-path restrictions. Provisioning now explicitly updates Homebrew and
checks its version before removing any package. A regression covers the old
version's rejection before package removal.

Successful run: [43167349273](https://github.com/ZsoltGajdacs/CodeCallNavigator/actions/runs/43167349273),
compatibility-fix commit `acc7da933471a772e663b0c0371b235bb0a2d0a1`.

- Official `xcode-27` GitHub-hosted arm64/macOS 27 runner.
- Homebrew 7.0.13 installed the checksum-pinned Graphviz 16.1.0 and libtool 2.6.2
  bottles; matching Graphviz/libtool/Aviator source archives were verified.
- Python 3.13, .NET 10, JDK 25 and Rust 1.99.1 setup passed.
- Analyzer bootstrap, complete locked Cargo source fetch and all 40 focused
  regressions passed on the hosted runner.
- Complete app/engine/source release packaging passed, including exact upstream
  revision/patch checks, archive checksums, strict/deep code signature validation
  and packaged engine `--help`.
- Artifact upload passed. `macos-release` artifact ID `13920893239`,
  372,882,894 bytes; reported artifact digest
  `sha256:5378a6bc07fe3918508abc2fe51a1381ff1a0abee1a4d62c9eacd2826e979869`.
- The publish job was **skipped**, as required for manual verification. No tag,
  GitHub release or test release was created. Publication sequencing was tested
  through the helper's behavioral tests; its first actual tag-triggered
  publication remains the owner's next release.

The runner image is a public preview; this workflow does not add Apple
notarization or Linux support. Pinned inputs remain checked-in in the source and
native-bottle locks. See [release usage](../../docs/public-releases.md).

## Independent artifact inspection

Downloaded the successful run's artifact. The initial large transfer was reset
by the storage connection; a resumable range download completed and matched the
reported artifact SHA256 above before extraction.

- Exactly the four expected release assets were present.
- All three release archives match their entries in `SHA256SUMS`.
- App and engine source identities both identify the tested `acc7da9` commit;
  the app reports a clean source tree and the pinned native versions.
- The app contains the AGPL text and original NuGet third-party notices.
- All 23,613 source-manifest file hashes match the tarball contents, including
  full pinned Graphviz/libtool/Aviator source archives and all upstream bundles.
- The project Git bundle restores the exact compiled commit and includes the
  workflow source.
- The downloaded application passes strict/deep code signature verification
  after extraction on macOS.

Artifact checks used temporary files only. Unrelated concurrent README/reference
and Rust UI work was preserved and excluded from this task's commits.
