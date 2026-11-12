# Public repository implementation verification

Date: 2026-11-12. Task: TASK-43, promoted from DRAFT-4.

Implemented AGPL-3.0-only licensing, structured bug/feature forms, private security
reporting guidance, solo-maintainer contribution/conduct policy, GitHub settings,
and matching-source release packaging. See
[the settings checklist](../docs/public-repository-checklist.md) and
[the release workflow](../docs/public-releases.md).

## Local checks

- All 87 Python tests pass, including 15 packaging and 6 source-release tests.
- Cargo metadata confirms AGPL-3.0-only for all three project crates.
- `cargo check --locked -p codecallnavigator --all-targets`: pass.
- `cargo test --locked -p codecallnavigator --all-targets`: pass; ignored tests
  remain ignored by this command.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`: pass.
- Both ignored `packaged_java` tests pass using real Maven/Gradle/JDK fixtures.
- Issue YAML parses, required field IDs are unique, blank issues are disabled,
  local Markdown targets resolve, and `git diff --check` passes.
- CodeScene reviews: all four modified Python source/test files score 10.0.
  Before committing, the safeguard found a small overall complexity regression
  in packaging. Extracting validation of one native dependency resolved it;
  all 21 focused packaging/source tests and the four-file safeguard then passed.
- Sonar is inactive: no `.sonar/config.json` in this repository.

The required `cargo test --workspace --all-targets -- --include-ignored` command
did **not** pass: the desktop binary stopped with 184 passing and 4 failing tests.
Three unchanged canvas benchmarks explicitly reject debug builds; saved/GPU
workloads also require an explicit copied profiling workspace. The ignored
process-group cancellation regression passed when rerun with permission to
manage its own subprocess group. The workspace-wide command did not continue to
later test binaries after the desktop failure. This is not a full Rust-suite pass
or an accepted waiver, and the task remains open for that gate.

## Actual release workflow

Tested the reviewed implementation from an isolated committed snapshot, without
committing or pushing the user's working branch. Snapshot commit:
`a331918eedf8d7189cb09681f1f33333cf567e1f` (verification-only, not a public tag).

`python3.13 tools/package.py --release` rebuilt the helpers, engine and desktop,
signed the app and produced ZIPs, a source tarball and `SHA256SUMS`. Initial
integration exposed epoch-dated license files unsupported by ZIP; timestamp
clamping and a regression fixed it. The final release run passed.

- All three downloadable artifact hashes match `SHA256SUMS`.
- All 23,613 source-manifest file hashes match archive contents.
- Project and all three upstream Git bundles restore offline; locked upstream
  revisions match and the Rune/CallAtlas patches apply successfully.
- Build manifests match packaged bytes; project AGPL text, NuGet MIT attribution
  and Roslyn third-party notices are present.
- Strict/deep app signature verification passes; packaged engine `--help` runs.

The temporary verification archive contains Cargo dependency sources and the
full pinned Graphviz, libtool and Aviator source archives. This demonstrates the
source-preservation workflow, not an entirely offline SDK build, bit-for-bit
reproducibility, or a published release. The final documentation additionally
links directly to the repository's release page.

## Live GitHub and publication boundary

REST API read-back confirms public visibility, public issues without interaction
limits, collaborator-only PR creation, Discussions off, description/topics,
private vulnerability reporting on, Dependabot alerts/security updates, secret
scanning and push protection. At the initial verification the remote repository
had no branches. Community files/forms/license detection awaited publication; code
scanning selection is deferred until GitHub can detect the published languages.
No release upload or Linux-support claim was made.

Detailed command logs for this run were saved under `/tmp/codecallnavigator-*task43.log`.
