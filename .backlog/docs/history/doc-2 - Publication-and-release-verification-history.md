---
id: doc-2
title: Publication and release verification history
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:07:00Z'
---
# Publication and release verification history


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-validation-md"></a>

## evidence/VALIDATION.md

Source: [evidence/VALIDATION.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/VALIDATION.md). Historical record; retained rationale, methods and limitations.

### Validation record — 2026-11-08

#### Delivery

`dist/CodeGraph.app` and `dist/CodeGraph.zip` were built with `tools/package.py`.
The executable is Mach-O arm64; deep strict ad hoc signature verification passes.
Pinned analyzer artifacts, relocated Graphviz/plugins/dylibs and third-party
notices are included. Production packaging rejects unexpected dependency revisions,
source changes and identity-patch checksum mismatches. SDKs/JDKs are not bundled.

With PATH restricted to `/usr/bin:/bin`, the packaged executable analyzed the
C# and Gradle fixtures together using bundled artifacts: 25 symbols and 26
relationships, successfully published to an isolated SQLite cache. It also rendered
the real mixed-project cache offscreen using bundled Graphviz. The packaged
interaction script rendered 331 GPU frames and exercised selection, directed paths,
pan/zoom, fit, keyboard focus, resizing and history. See `ui/packaged-interactions.png`
and `ui/packaged-real-overview.png`. Normal startup uses the user's workspace;
fixture data requires an explicit `--scene fixture` argument.

#### Analyzer acceptance

PacificFate and StoryEngine source files were not modified. Their applications and
tests were not run. Real restores/compilation were permitted and performed.

| Build | Indexed symbols | Indexed relationships |
|---|---:|---:|
| PacificFate solution | 20,494 | 75,881 |
| StoryEngine backend | 14,504 | 22,326 |
| Combined app pipeline | 34,998 | 98,207 |

C# uses pinned CallAtlas with an identity serialization correction, preserving
compiler assembly ownership. Java uses existing java-callgraph2 with BCEL 6.13.0:
JDT LS failed the mandatory StoryEngine import gate and was rejected before the
UI milestone. Java 21 and 25 fixtures pass. Engine distinctions and missing bodies
remain explicit, with no adapter call resolution or DI inference. Exact signatures,
manual source checks and observed limitations are in `analyzers/ACCEPTANCE.md`,
`analyzers/native-pipeline.md` and the analyzer summary JSON files.

#### Automated checks

- All-target Clippy with `--no-deps -- -D warnings`: passed.
- Rust: 34 tests passed, including the installed-JDK compiler regression normally
  marked ignored. Coverage includes graph paths/cycles/filters/budgets, real
  Graphviz edge identity, SQLite generations and failed refresh, descendant
  cancellation, freshness, pagination and native Rune interaction.
- Python: 20 tests passed, including rejected JDT responses/framing, actual Git
  bootstrap behavior and staged/unpatched dependency packaging rejection.
- CodeScene staged gate: passed, 45 eligible files checked with no issues.
  Introduced findings were refactored; final affected-file reviews score 10.
  See `code-health.json`. The original unborn-HEAD error was resolved with a
  metadata-only initial commit before checking the entire staged application.
- Sonar remains inactive because this new project has no `.sonar/config.json`.

#### Scale measurement

Release benchmark: 250,000 method nodes and 2,000,000 relationships, 100 queries per
kind (including ten search prefixes matching all methods).

| Measurement | Result | Target |
|---|---:|---:|
| Cached graph open | 0.633 s | <5 s |
| Search p95 | 99.0 ms | <200 ms |
| Neighbor query p95 | 0.005 ms | <200 ms |
| Publication | 2.653 s | — |
| Peak process footprint | 488,440,648 bytes | — |

Hardware available here is an Apple M4 Pro with 48 GB RAM. This verifies the target
workload and latencies on that machine; the specified physical 32 GB machine has
not been tested. Raw measurements are in `performance/scale-final.json` and
`performance/scale-final-resources.txt` (the earlier run is also retained). Layout is asynchronous and excluded from cached
opening/query timing.

#### Remaining physical validation

Headless Rune Metal rendering, light/dark screenshots and scripted interaction
pass. A live native-window probe rendered its first frame, then failed to obtain
a display link (-6661). CoreGraphics reports successful enumeration with **zero
active displays** in this execution session. See `ui/native-display-status.json`.
The partial native trace/log are failed-probe evidence, not a live interaction pass.

Physical trackpad navigation, keyboard focus and resizing in a visible macOS window
remain unverified. TASK-5 stays In Progress; neither this check nor the exact 32 GB
hardware benchmark is claimed complete. On a connected display, open the delivered
app, select a build, verify pan/pinch/scroll/fit, Tab/Cmd-F/Cmd-O and resizing; rerun
the release benchmark on a 32 GB Apple Silicon Mac.


<a id="source-evidence-ci-tag-release-md"></a>

## evidence/ci/tag-release.md

Source: [evidence/ci/tag-release.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ci/tag-release.md). Historical record; retained rationale, methods and limitations.

### Tag-triggered macOS release workflow

Task: TASK-44. Date: 2026-11-12.

#### Delivered behavior

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

#### Local verification

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

#### Hosted integration

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
native-bottle locks. See [release usage](<../../../docs/public-releases.md>).

#### Independent artifact inspection

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


<a id="source-evidence-public-repository-md"></a>

## evidence/public-repository.md

Source: [evidence/public-repository.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/public-repository.md). Historical record; retained rationale, methods and limitations.

### Public repository implementation verification

Date: 2026-11-12. Task: TASK-43, promoted from DRAFT-4.

Implemented AGPL-3.0-only licensing, structured bug/feature forms, private security
reporting guidance, solo-maintainer contribution/conduct policy, GitHub settings,
and matching-source release packaging. See
[the settings checklist](<../operations/doc-16 - Public-repository-maintenance-and-publication-checklist.md>) and
[the release workflow](<../../../docs/public-releases.md>).

#### Local checks

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
later test binaries after the desktop failure. This was not a full Rust-suite
pass; at that checkpoint no waiver existed and the task remained open.

#### Actual release workflow

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

#### Live GitHub and publication boundary

REST API read-back confirms public visibility, public issues without interaction
limits, collaborator-only PR creation, Discussions off, description/topics,
private vulnerability reporting on, Dependabot alerts/security updates, secret
scanning and push protection. At the initial verification the remote repository
had no branches. Community files/forms/license detection awaited publication; code
scanning selection is deferred until GitHub can detect the published languages.
No release upload or Linux-support claim was made.

Detailed command logs for this run were saved under `/tmp/codecallnavigator-*task43.log`.

#### Requested commit, push and live verification

On 2026-11-12 the owner explicitly requested publication. After the passing
four-file CodeScene safeguard, committed the 22 approved files as
`fcc74a8d0ef2f53ae8a79651e52363eaf432d812` and pushed `HEAD:main` without force.
GitHub's default branch read back with that exact commit.

The public Community Standards UI marks README, code of conduct, contributing,
license, security policy and issue templates Added. The license page displays
the GNU AGPLv3 summary. Both public YAML previews render their intended forms and
required-field markers without template errors. The eight published community
and template blobs match the commit exactly. Private reporting is enabled and
the security policy URL resolves in GitHub metadata.

`Repository.isBlankIssuesEnabled` reports viewer permission, not the raw config
value: it is true for the owner. The published config is false, and GitHub
documents the maintainer exception. REST community-profile `issue_template` is
null and GraphQL `issueTemplates` is empty for these YAML forms; the public
community UI and rendered previews are the relevant evidence. The composer
itself required browser sign-in, so no issue submission was tested or created.

The publication itself did not assert a full-suite pass or waive that gate.

#### Accepted verification exception

On 2026-11-12 the owner explicitly accepted the remaining full ignored Rust-suite
limitation for TASK-43 ("Accept this for now"). All task acceptance criteria and
live GitHub verification are complete; this exception allows the task to close
without claiming that the interrupted workspace-wide test run passed.

The owner also requested that AGENTS.md make runtime verification conditional on
runtime impact. Documentation, issue templates, licensing text and descriptive
metadata now use content/syntax/link/diff checks; runtime-affecting code,
dependency, bootstrap and packaging changes retain their applicable checks.
This final policy/evidence update changes no runtime behavior, so no runtime
tests were rerun.
