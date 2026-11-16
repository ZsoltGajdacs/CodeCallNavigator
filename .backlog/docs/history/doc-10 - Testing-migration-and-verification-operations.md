---
id: doc-10
title: Testing migration and verification operations
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:07:00Z'
---
# Testing migration and verification operations


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-testing-strategy-distribution-md"></a>

## evidence/testing-strategy/distribution.md

Source: [evidence/testing-strategy/distribution.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/distribution.md). Historical record; retained rationale, methods and limitations.

### Current test distribution

Counts are executable registrations, kept separate from standalone harnesses.
Acceptance means end-to-end coverage through a public application boundary.

| Component | Unit | Integration | End-to-end | Performance |
| --- | ---: | ---: | ---: | ---: |
| UI | 174 | 247 | 0 | 3 |
| CORE | 109 | 109 | 0 | 0 |
| ENGINE | 50 | 233 | 81 | 0 |
| MCP | 16 | 16 | 4 | 0 |
| TOOLING | 58 | 258 | 0 | 0 |
| **Total** | 407 | 863 | 85 | 3 |

| Standalone harness owner | Integration | End-to-end | Performance |
| --- | ---: | ---: | ---: |
| UI | 0 | 2 | 0 |
| CORE | 0 | 0 | 1 |
| ENGINE | 1 | 5 | 1 |
| MCP | 0 | 1 | 0 |
| TOOLING | 0 | 0 | 0 |

Total: **1,358 registrations and 11 standalone harnesses**.

A harness may contain many scenarios; its count is not comparable to a single test.
Rust lib/bin registrations and inherited Python cases count separately where they
are actually registered. Fixture loading alone does not determine the category.

The thirteen classification corrections and legacy OS simulation limits are
recorded in [the source review](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/classification-review.json). The placement mappings
preserve existing assertions; no tests were added merely to create a pyramid ratio.
The pyramid guides future responsibility, rather than making current ratios an
acceptance gate.

Inventory SHA-256: `15c74b6be5d4586be77f7ae60137c580457c263cbd6311fc7fa123f4ea8c1b93`.


<a id="source-evidence-testing-strategy-migration-md"></a>

## evidence/testing-strategy/migration.md

Source: [evidence/testing-strategy/migration.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/migration.md). Historical record; retained rationale, methods and limitations.

### Testing strategy migration evidence

The migration remains on the existing branch and preserves all existing test
function definitions. No test deletion or claim of equivalent replacement coverage
is recorded in this batch. `baseline.json` captures source/executable counts,
observed machine/tool versions, historical reports and current category observations.
Those reports have different selections/cache conditions and are not directly
comparable full-suite benchmarks.

#### Review refinements

The source/helper audit in `classification-review.json` identified thirteen
registrations that exercise a real file worker or process/protocol orchestration
and fault probes. They move from unit to integration without changing their test
bodies. Inherited successful OS protocol simulations are explicitly identified;
they do not prove successful Homebrew installation, screenshot capture or native
window mutation. Real packaging and native journeys remain separate coverage.
No mocked analyzer or graph decision is introduced by this work.

`core-placement.json` maps a further 72 core-owned registrations (32 unit, 40
integration) into the core package, preserving 264 assertion macros. Twelve files
move with only namespace changes and required formatting; three core cases and
their graph fixture move out of the mixed entry-metadata target. The existing real
C# compiler case stays in the root package so its manifest-relative fixture paths
are unchanged. Together with the first eleven flow cases, this removes desktop
and Rune dependencies from the core component's test selection. The focused core
unit run passed 109 registrations; its compiler artifact records contain only core
test targets, with 9.597 seconds compilation and 8.087 seconds execution.

Serialization is now independent of category. Source inspection found concrete
shared resources in the private Java checkout, repository MediatR solution and
profiling machine/store. Their seven Rust registrations have explicit serial
groups. Other correctness groups use bounded four-worker parallelism; unit groups
use libtest defaults. Artifacts remain sequential, and Python remains serial
because its bootstrap/CI cases mutate interpreter globals. A category label alone
no longer serializes a Rust target.

The blocked-output acceptance probe originally assumed a pipe queue byte threshold
that does not establish backpressure on this Mac. Its replacement uses a real
bounded Unix socket, suspends draining only after a real initial CLI frame, fills
the physical transport and checks kernel write-readiness before real compiler
work. Filler bytes are never decoded as business frames. The shutdown deadline,
successful exit and descendant checks remain intact. The focused packaged-CLI run
and all fourteen scenarios in the final full verification passed.

Cleanup regressions exercise actual success, failure, timeout, SIGTERM during a
running child and between-stage recording, active held files, ownership rejection
and explicit retention. Already-cleaned reclamation now preserves the original
cleanup receipt instead of overwriting its reclaimed-byte count with zero.

#### Coverage mapping

| Prior execution | New execution | Retained verification |
| --- | --- | --- |
| Ordinary Rust suite in debug, repeated by the full release command with `--include-ignored` | Each exact executable registration is assigned one category; ordinary tests execute there once | Same test bodies/assertions, runtime registration checking and real dependencies. A documented release requirement selects the proper profile rather than rerunning the whole suite. |
| Ignored tests bundled with ordinary tests and canvas benchmarks | Explicit integration, acceptance and performance categories with requirements | Compiler/build, CLI/MCP, native UI, storage and CPU/GPU checks remain selected by their actual boundary and required environment. |
| TypeScript `observations.test.js` two in-memory cases run with all compiler tests | Two exact names run in unit; remaining 110 Node cases run in integration | `different values observed during preparation are rejected immediately` still checks inconsistent repeated observations; `unchanged observations may be queried repeatedly and verified` still checks stable repeated reads. Same production `Observations` implementation and original assertions. Filesystem changes and compiler behavior remain real integration coverage. |
| Root-package `tests/flow_core.rs`, compiled with desktop dependencies | Core-package `crates/codecallnavigator-core/tests/flow_core.rs` | All eleven cases and 45 assertion macros are preserved. Ten remain unit cases; the real Graphviz layout case remains integration coverage with its Graphviz requirement. Only the crate namespace import changes. |
| Full verification's mixed release acceptance/benchmark timing | Separate compilation, setup, execution, cleanup and other stage costs | Functional acceptance and performance failures remain failures; exceeding a timing aspiration does not fail a correct run. |
| Cleanup only at composite end | Cleanup after every category, including standalone invocation and failure paths | Existing ownership/inactivity protections, process reaping, downloaded-cache/evidence retention and explicit diagnostic retention remain required. |

Source-case identity and executable registration are separate: Rust lib/bin targets
may register one source case in two executables; Python inherited test methods can
register in multiple concrete classes. Both executable registrations remain covered.
The earlier conversation's 1,071 source estimate omitted Rust integration tests in
the root `tests/` directory and cannot serve as the complete inventory baseline.

#### Core flow placement batch

The flow target now belongs to the core crate that implements the exercised
operations. Running `cargo test --locked -p codecallnavigator-core --test flow_core`
therefore does not compile the desktop, Rune or the engine merely to test shared
flow rules. The target intentionally retains its real Graphviz case; moving files
does not turn that boundary test into a unit test. Category scripts still select
the ten unit cases and the Graphviz integration case separately.

[The placement proof](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/flow-core-placement.json) records every old-to-new executable
identity and SHA-256 hashes. Byte comparison confirmed the destination equals the
original file after the single `codecallnavigator::` → `codecallnavigator_core::`
namespace substitution. Every helper, test body and assertion is unchanged; no
test was removed, replaced or reclassified. Category counts are unchanged.

This is a dependency-placement improvement, not a measured timing claim. The
previous unit run exercised the old registrations; post-move focused execution
must establish the new target's compiler/runtime result. Timing and cleanup
outcomes belong to that actual verification report.

#### Remaining core test placement batch

A further 72 registrations (32 unit, 40 integration) move from root-package
targets to the core crate: twelve complete test files and the three core cases
from `entry_metadata.rs`. Its shared `graph()` helper moves with those three
cases. The real C# entry-metadata analyzer case remains in the root package;
its body and manifest-relative fixture paths are unchanged. No core-owned Rust
test registration remains attached to the desktop package after this batch.

[The complete mapping and preservation proof](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/core-placement.json) records all
72 old/new executable identities, per-file or per-item hashes, and all 264 retained
assertion macros. Categories, requirements, purposes and test bodies are preserved.
Twelve whole files initially matched their original bytes after only replacing
`codecallnavigator::` with `codecallnavigator_core::`. Four then required rustfmt
wrapping; their final bytes were compared with deterministic rustfmt output of
those exact namespace-only intermediates. Both intermediate and final hashes are
recorded. This is not whitespace stripping, which could hide changed string data.

Core unit/integration scripts can now compile the owning core package without
pulling the desktop/Rune/engine merely because tests were placed in root targets.
No test is removed or reclassified by this placement batch. The focused core run
and final full/light category executions passed the new targets. Earlier full-run
reports retain their original selected identities, linked
to the new registrations through the preservation mapping.

#### Private Java fixture build scope

Only these two existing engine integration registrations append
`-DskipUi` to their child process's `MAVEN_OPTS`:

- `codecallnavigator-engine/lib:codecallnavigator_engine::analyzers::java::compiled_acceptance::recurring_billing_successful_build_outputs_preserve_both_generated_copies`
- `codecallnavigator/test:java_duplicates::recurring_billing_retains_duplicate_generated_declarations`

The private fixture's inspected Maven configuration supports the `skipUi` property:
its `no-ui-build` profile activates on that property and its frontend Ant target
has an `unless="skipUi"` guard. The flag excludes unrelated frontend compilation
and Vitest work from these Java analyzer checks. Real Java compilation, generated
outputs, duplicate declaration checks and every existing assertion remain intact.
No private checkout files or global environment settings are changed. Other Maven
tests do not receive the flag.

The inventory's optional `environment_append` mapping defaults to empty and
validates environment names and string values before execution. The runner must
append the flags to inherited child values rather than replace existing Maven
options. Invalid mapping shapes, names, non-string values and embedded NUL values
are rejected by the existing metadata regression test.

Both original Java cases passed in the focused engine integration and final full
verification. The flag changes only those child environments. Observations across
different suite selections and cache conditions do not establish a controlled
full-suite speedup.

#### Archived C# fixture temporary ownership

The raw-flow and ingestion gates previously recreated archived physical fixture
paths beneath `/private/tmp/task22-flow-baseline-source` and
`/private/tmp/task22-ingestion-baseline`. Their project/build outputs therefore
escaped the category's owned `TMPDIR`, process-use checks and cleanup accounting.
Future runs materialize identical source/project bytes beneath each gate's
existing `TemporaryDirectory` work instead; old fixture directories are neither
adopted nor deleted.

[The expected-data relocation proof](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/csharp-fixture-ownership.json) records the
only expected snapshot changes: 27 physical-path/identity strings across the
three ingestion native projections and one raw-export architecture source path.
Rebasing requires an exact old-root value or a complete old-root-plus-slash prefix;
similar names and embedded source text remain unchanged. Canonical projections
and historical artifacts remain exactly as archived. Actual compiler output is
never normalized, and complete projection/field comparisons, source-byte checks
and flow determinism assertions remain in place.

Five focused regressions exercise prefix boundaries and snapshot immutability,
real materialization of all three encodings, source SHA-256 preservation, owned
raw-export source paths, and refusal of symlinked or escaping source/project
filenames. They do not fabricate compiler success or establish passing compiler
acceptance. Both actual compiler gates subsequently passed, including their final
full-composite invocations.

#### Observed execution

`target/verification/unit-f2lmlps5/results.json` records a completed unit run:
48.965 seconds compilation, 2.429 setup, 57.521 test execution and 2.393 cleanup.
`target/verification/unit-_sj7s_5c/results.json` records the smaller Python-only
observation at 9.850 seconds execution. These are observations of the selected
revision/tests at those runs, not promises for later inventory additions or a
certified cold/warm benchmark. Inspect each retained `selection.json` for exact
selection; do not infer passing-test counts from selected registrations.

Historical `full-l9fmsdlu` recorded 226.394 seconds ordinary Rust execution and
1774.885 seconds combined release acceptance/benchmarks. `full-k7cvs5hs` recorded
202.976 and 1762.168 seconds respectively. These figures identify prior cost centers,
not the speedup of the new complete full composite. Historical machine/cache state
was not recorded; current machine metadata in `baseline.json` is labelled as current.

#### Final validation

The pure reporting regressions verify that failed-stage execution and cleanup costs
remain visible, selected/omitted cases and gates stay separate, source-case counts
differ from executable registrations, and interrupted/not-run stages never become
fabricated passing-test counts. `tests/test_verification.py` separately checks the
shared category plan, isolated build paths, retained package/install/agent journey
contracts and wrapper dry runs without evidence writes.

[The durable validation record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/testing-strategy/validation.json) records:

- `full-enbwvtxc`: all 112 stages passed, including real compiler/CLI/MCP,
  installed packages/signatures, native light/dark journeys, Metal and storage
  benchmarks, CodeScene and cleanup. Python integration ran 258 tests without skips.
- The full selections cover all 1,358 registrations and eleven standalone harnesses
  exactly once, with no missing or duplicate identities across categories.
- `light-9t8sexpr`: all 83 stages passed; 407 unit and 635 ordinary integration
  registrations selected, with compiler-backed/acceptance/performance omissions
  explicitly reported.
- Full unit execution took 17.795 seconds; compilation took 61.037, setup 0.958
  and cleanup 4.962 seconds. This is an observation, not a timing acceptance gate
  or a certified cold/warm benchmark.
- All four test-category cleanups passed and reclaimed an estimated 8,941,891,272
  logical bytes. Required packages/rollback evidence remain; no category build
  or external temporary workspace remains for that full run.
- CodeScene reviewed all 51 changed source files successfully. Tool versions are
  recorded in the full summary, baseline and `toolchains.json`. SonarMCP is inactive
  because this project has no `.sonar/config.json`.

The final native gate launches are proven beneath their category-owned scratch
directory, which is absent after cleanup. Failed/interrupted native fixtures are
token-bound and moved into retained gate evidence only after inactivity checks.
Real-file tests cover active handles, unsafe bindings, incomplete tokens and
interrupted record writes; failure reports include measured retained bytes and
unknown-size paths. A transient inconclusive native inspection in an earlier
attempt correctly preserved its build; a later independent idle proof allowed
guarded reclamation. That earlier full attempt remains recorded as failed.

Before removing any future expensive test,
record its identity, defect scenario, assertion-to-retained-test mapping, measured
cost and demonstrated failure detection. Keep unique boundary coverage even if slow.


<a id="source-evidence-verification-2026-11-12-readme-md"></a>

## evidence/verification/2026-11-12/README.md

Source: [evidence/verification/2026-11-12/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/2026-11-12/README.md). Historical record; retained rationale, methods and limitations.

### Full/light verification and artifact cleanup

The final configured native `verify-full.sh` invocation completed all 50 stages
successfully on 2026-11-12. See [summary.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/2026-11-12/summary.json) for machine-readable
counts. Detailed local numbered logs, results.json and cleanup.json are retained
at `/private/tmp/codecallnavigator-verify-full-final/`.

#### Delivered behavior

- `verify-light.sh`: changed-project-file formatting/syntax, regular workspace
  check/Clippy/tests, both Python suites and real CodeScene reviews/safeguard.
  Documentation-only changes use static checks and cleanup without runtime tests.
- `verify-full.sh`: explicit prerequisite checks, pinned analyzer bootstrap,
  TypeScript tests, matching debug/release engine builds, all release acceptance
  tests including ignored tests, isolated saved-workspace CPU/GPU benchmarks,
  storage benchmark, local packaging/signature checks and CodeScene.
- Checkout locking prevents competing verification runs. Stages have time limits
  and logs; failures remain failures, later ordinary stages remain not run, and
  cleanup runs as an always-finalizer. Live stages report running.
- Full cleans dev/release profile artifacts only after build stages were reached.
  Light cleans incremental state while keeping compiled dependencies. Both
  remove their temporary profiling/scale stores and old rotated app backups,
  retaining current packages, reports and the newest rollback per app name.
- Native lsof usage checks and path/symlink guards protect active artifacts and
  paths outside the selected repository/run. Cleanup failures fail the run;
  `--keep-artifacts` explicitly retains artifacts for investigation.
- Verification disables incremental compilation and enables Cargo's own aged
  dependency-cache cleanup. No shared Cargo-cache directories are manually purged.

Setup, mode selection and flags are documented in
[the validation reference](<../testing/doc-21 - Verification-prerequisites-and-operations.md#validation>).

#### Verification

- Ordinary Rust workspace: **494 passed**, 84 intentionally ignored by this stage.
- Full release workspace: **578 passed**, **0 ignored**, including all three
  release-only canvas benchmarks and real compiler/build acceptance tests.
- Python: **154** tests under tests/ and **26** under tools/ passed in the full run.
- TypeScript: **112 passed**, no skipped tests.
- Final focused orchestration/MCP/cleanup suite: **49 passed**. It covers real Git
  change selection, process failures/timeouts and cleanup limitations, lock
  contention, SQLite WAL snapshots, source-review coverage, mode/stage selection,
  relative input paths, symlink escapes, active-artifact retention and deletion.
- Native Metal offscreen benchmarks used an isolated SQLite backup of a real
  saved workspace. Storage benchmark covered 250,000 nodes and 2,000,000 edges;
  all defined acceptance targets passed. Offscreen timing is not displayed FPS.
- Local app/engine packaging, signature verification and packaged-engine startup
  passed. No release was published.
- All 29 reviewed source files scored **10.0**; CodeScene safeguard passed with
  29/29 eligible files checked and no findings. Sonar is unconfigured/inactive.
- Independent reviews found no remaining blocking issues. Diff whitespace check
  passed. No commit, push or pull request was created.

The full cleanup succeeded with no blocked/uninspectable candidates and removed
an estimated **20.45 GiB** of generated artifacts. Cargo reported 14.6 GiB of dev
and 1.3 GiB of release artifacts removed; additional removals were old app backups
and run-local benchmark stores. Current packages and rollback copies remain;
`target/` was reduced to 8 KiB and `dist/` to 584 MiB. Estimates count file sizes,
so they are not a measurement of physical free blocks on APFS.

#### Reliability fixes discovered by the verifier

The engine cancellation fixture now atomically publishes its PID marker instead
of exposing an empty file before its contents are written. The private compiled
Java acceptance test now obtains fresh real-build metadata through its owning
Maven adapter when no explicit metadata fixture is supplied. Explicit supplied
metadata remains supported. This removes a previously manual full-suite input.
The shared build-label helper now uses a typed internal kind to satisfy the
safeguard without changing the public BuildTarget contract.

Earlier debug/sandbox or partially configured failures are superseded by the
successful final native full run. Required full inputs remain explicit; no checks
were waived or silently omitted to obtain this result.


<a id="source-evidence-verification-cache-clearing-md"></a>

## evidence/verification/cache-clearing.md

Source: [evidence/verification/cache-clearing.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/cache-clearing.md). Historical record; retained rationale, methods and limitations.

### Project cache clearing verification

Implemented for TASK-36. The desktop calls the engine machine protocol; both it
and `codegraph-engine clear-cache --project PATH` use one core operation under
`AnalysisLock`. Only inference.sqlite3 and its SQLite sidecars are removed.
Canonical graph, configuration, sources, navigation and build outputs remain.
Protocol version 1 gains additive clear_cache/cache_cleared messages; older engines
reject the unsupported request explicitly.

#### Passed

- `cargo check --locked -p codegraph --all-targets`
- `cargo test --locked -p codegraph --all-targets`
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`
- `cargo test --locked -p codegraph-core -p codegraph-engine --all-targets`: passed on full retry.
- Core cache clearing regressions: corrupt cache and sidecars, idempotence,
  preserved unrelated files, held lock, symlink and hard-link rejection.
- `cargo test --locked -p codegraph-engine --bin codegraph-engine --test clear_cache`:
  5 entrypoint tests and 4 real executable tests, including CLI/protocol success,
  absent cache, invalid projects, lock conflicts and unsafe cache paths.
- `cargo test --locked -p codegraph --bin codegraph ui_tests::cache_clear -- --include-ignored`:
  3 native runtime tests covering adjacent controls, busy guards, real engine
  clearing, preserved visible graph, success feedback, failure and retry.
- `cargo test --locked -p codegraph --test incremental_inference cached_inference_matches_full_analysis_after_compiler_facts_change -- --ignored --nocapture`:
  real TypeScript compiler acceptance verifies warm cache hits, CLI clearing,
  preserved artifact, zero subsequent hits and identical canonical graph bytes.
- CodeScene reviews: modified source files have no findings; scored files 10.0.
  Pre-commit safeguard passed with 16 eligible files checked and no regressions.
- `python3.13 tools/package.py`: packaged and signed `dist/CodeGraph.app`.
- `git diff --check`.

#### Full validation limitations

`cargo test --workspace --all-targets -- --include-ignored` failed in four
unchanged desktop tests: canvas_gpu_frame_completion,
dense_canvas_frame_preparation and saved_canvas_frame_preparation explicitly
require release mode; forced_cancellation_reaps_engine_and_its_separate_child_group
failed its detached-process cleanup assertion in the macOS sandbox. Cargo stopped
at that test binary, so later ignored workspace suites were not verified by this
command. These failures are outside the cache-clearing change.

An additional core/engine test run encountered the unchanged process deadline test
zero_deadline_never_starts_process_and_cancellation_takes_precedence failing its
marker assertion; its immediate isolated retry and subsequent full core/engine
retry both passed.

Sonar is inactive because this project has no `.sonar/config.json`.
The change is committed locally without pushing. TASK-36 remains In Progress
because the full validation limitations have not been waived.


<a id="source-evidence-verification-vendor-refresh-2026-11-11-md"></a>

## evidence/verification/vendor-refresh-2026-11-11.md

Source: [evidence/verification/vendor-refresh-2026-11-11.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-11.md). Historical record; retained rationale, methods and limitations.

### Vendor refresh, 2026-11-11

Fetched all three Git source dependencies and queried their upstream default branches:

| Source | Branch | Verified tip |
| --- | --- | --- |
| Rune | dev | bba30a61cfcec6683e3fdedf156a92dae89d8b04 |
| CallAtlas | main | 916c2419c0cdb91b5b86b84df4842dadb17bb6b4 |
| java-callgraph2 | main | 77f566d0878958bab929b4cdeafcc2c762ff555e |

Only Rune advanced. Before switching revisions, its complete diff and CallAtlas's
complete diff were verified against their checksum-pinned patches. Rune's patch
was rebased, preserving new visibility callbacks and moving observer wiring to
the upstream virtual desktop module. All migration checkpoints now target the new
base; see `patches/README.md` for application order and old-base migration.

The combined patch makes `CommitMeter::handler` and `frame_ends` debug-only,
matching their callers. Three additional warnings introduced upstream were fixed
by narrowing module-local helper visibility. Cargo.lock gained two already
resolved Rune dependency edges (`skrifa`, `windows-core`). Downloaded investigation
binaries retain their separately pinned versions.

#### Verification

- Final `cargo check --locked -p codecallnavigator --all-targets`: passed, no warnings.
- `cargo test --locked -p codecallnavigator --all-targets`: 329 passed, 69 ignored.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`: passed.
- `python3.13 tools/bootstrap.py`: passed (CallAtlas and Java adapters rebuilt).
- `python3.13 tools/package.py`: passed; release engine and desktop built without
  compiler warnings, signed application and ZIP produced in `dist`.
- `python3.13 -m unittest discover -s tests -v`: 76 passed.
- Dependency patch tests: 7 passed, including migration and unknown-edit rejection.
- All 4 debug input observer regressions passed.
- Release `ui_tests::canvas_performance::dense_canvas_frame_preparation`: passed.
- Independent implementation review and patch/checksum verification passed.
- `git diff --check`: passed.

#### Limits

The exact required `cargo test --workspace --all-targets -- --include-ignored`
command fails because three ignored performance tests explicitly require release
mode; two also require a copied saved profiling workspace. A continuation
excluding those three passed compiler-backed ASP.NET, flow, incremental inference,
Maven and Gradle checks, then stopped at the private billing fixture test:
`recurring_billing_retains_duplicate_generated_declarations` requires
`CODECALLNAVIGATOR_RECURRING_BILLING`, which is unset. Subsequent acceptance
binaries were not run by that command. Full workspace acceptance is not claimed.

CodeScene reviewed the six source files changed by conflict resolution or warning
fixes. Scores: app 7.32, window 8.39, virtual desktop window 8.96, commit meters
9.53, motion flights 7.61, press repeat 10. Existing findings concern unchanged
upstream logic; review found no reason to broaden this vendor update into those
refactors. The root pre-commit safeguard passed with no eligible source files
(the root tracks patches rather than vendor source). Sonar is unconfigured.

Task TASK-41 retains the full-validation limitation; no waiver was assumed.


<a id="source-evidence-verification-vendor-refresh-2026-11-13-md"></a>

## evidence/verification/vendor-refresh-2026-11-13.md

Source: [evidence/verification/vendor-refresh-2026-11-13.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-13.md). Historical record; retained rationale, methods and limitations.

### Vendor source refresh — TASK-64

#### Source selection and integration

Fetched the configured upstream repositories on 2026-11-13. Rune `dev` advanced
from `bba30a61cfcec6683e3fdedf156a92dae89d8b04` to
`c4eda05889c984fff91ccd906e3c00cdfc4cae24` (338 non-merge commits).
CallAtlas `main` remains `916c2419c0cdb91b5b86b84df4842dadb17bb6b4`;
java-callgraph2 `main` remains `77f566d0878958bab929b4cdeafcc2c762ff555e`.
Downloaded investigation binaries were outside this Git-source refresh.

Verified the complete original vendor diffs against the checksum-pinned patches.
Reversed only the exact old Rune patch, checked the checkout was clean, moved to
the new revision, and installed the rebased patch. No reset or stash was used.
All four Rune patch alternatives were rebased and their checksums refreshed;
they remain alternative combined patches, never patches to stack.

Observer wiring preserves the new `on_rest`, browser CPU/GPU startup, IME and
direction-chord behavior. Pointer observers precede autoscroll early returns.
The obsolete focus-method warning hunk was removed. The oriented-segment GPU
regression uses the updated metallib API; bytecode fingerprints were not changed
to match stale libraries. Runtime shader compilation remains intentional.

The consuming workspace now enables Rune's `tooling` feature so release
headless acceptance and packaged captures retain their behavior. It selects
Rune's upstream-patched wgpu-hal, winit, parley_engine and accesskit_windows
through workspace overrides; tiny-skia-path is already an upstream path dependency.
The root lockfile records these paths. No other root application API adaptation
was necessary for the all-targets compilation.

Rune's required integration changelog introduced an added file into the patch.
The migration fixture previously assumed every patch path existed upstream and
failed two setup cases. It now seeds pristine files from old-side paths and
uses the production whole-checkout comparison, including untracked added files.

#### Verification

- Workspace all-targets `cargo check` passed.
- Seven Python dependency patch tests passed after the fixture correction.
- Independent pristine-source checks passed all four patch checksums and clean
  applications, checkpoint-to-current migration, repeat verification, and
  rejection/preservation of unrelated shader edits.
- Independent static review found no actionable rebase defects. Installed Rune
  HEAD matches the lock and its complete checkout diff matches the current patch.
- CodeScene reviewed the 18 touched Rust files in the rebased dependency and the
  migration fixture. The fixture scored 10.0; dependency findings concern upstream
  complexity, including existing input/startup functions. No broad upstream
  refactoring was introduced during this refresh.
- The focused upstream Rune test
  `oriented_segment_quads_preserve_caps_clips_and_layer_pixels` passed with native
  Metal access (1 passed, 2,184 filtered out). It checks pixel equivalence at 1x
  and 2x and rejection of stale precompiled shader bytecode.
- `./verify-full.sh --base origin/main` passed all 27 stages outside the macOS
  sandbox. Local logs and per-stage results are retained under
  `target/verification/full-e45ldvgw/`.
- Ordinary Rust: 674 passed, zero failed, 110 ignored; release acceptance:
  **784 passed, zero failed, zero ignored**. Both private Recurring Billing
  cases ran and passed. Storage and CPU/GPU benchmark stages passed.
- Python: 193 tests passed; tools suite ran 50 with one optional compiler
  journey skipped. That journey was then run explicitly with the real release
  CLI and TypeScript modules and passed (see `optional-freshness-journey.log`).
  All 112 TypeScript tests passed.
- Bootstrap, strict Clippy, package signatures, installed CLI, independent MCP
  installation, and real CLI/MCP compiler journeys passed. CodeScene's safeguard
  checked the changed project Python source and passed. Sonar is unconfigured.
- Packaged release headless captures completed in isolated temporary app stores
  in dark 1420×900 and light 1050×680 logical sizes. This also exercises the new
  explicit release `tooling` feature. See current [dark](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-13/fixture-dark.png)
  and [light](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-13/fixture-light.png) captures and the previous
  package's [dark](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-13/baseline-dark.png) and
  [light](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/verification/vendor-refresh-2026-11-13/baseline-light.png) captures. Visual inspection
  found the filter/card overlap already present in the previous package;
  this refresh does not fix that existing scene/layout limitation. The captures
  are not asserted to be pixel-identical.

Compilation emits upstream warnings in Rune and its nested tiny-skia-path
dependency. The standalone Rune test build also reports stale precompiled
shader libraries and other upstream test warnings. This is not a warning-free
upstream build. Shader freshness guards deliberately reject the bytecode for the
modified WGSL; upstream precompiled-library freshness tests are not claimed.

Browser CPU-first startup/handover is statically reviewed, not exercised in a
browser. Native tests and offscreen GPU measurements do not establish behavior
on Windows, Linux or web, nor native displayed-frame latency.
