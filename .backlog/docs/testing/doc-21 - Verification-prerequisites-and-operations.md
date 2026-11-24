---
id: doc-21
title: Verification prerequisites and operations
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-23T21:36:00Z'
---
# Verification prerequisites and operations

Migrated 2026-11-16 from `docs/reference.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Validation

Agents must use the deterministic changed-file entrypoint, which works from any
current directory when invoked by its absolute path:

```sh
./verify-changes.sh
./verify-changes.sh --format json
./verify-changes.sh --preflight
./verify-changes.sh --phase development --run
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

The script reports by default and executes the build and test plan only with `--run`.
Use `--preflight` to check the same selected inputs without building or creating an evidence directory.
A focused final run checks all selected suite inputs together before any compilation.
For plans that need Graphviz, the selector reads `analyzers.lock.json` and uses a matching
installed Homebrew version when the default `dot` differs. The selected version also applies
to package and test subprocesses. Configure the explicit fixture and profile inputs below.

Agents use
`--phase development --run` during implementation and fixes. It runs the
applicable unit tests and their build/setup/cleanup only. An empty unit selection
creates no evidence directory. Development evidence names its phase and identifies
checks reserved for final verification.

After the last edit, use `--run` to run the complete final plan. Fix failures and
rerun that plan after iterating with units. The script determines full final
verification automatically from explicit file rules, including conservative full
fallback for unknown paths.
Verification-tooling and selector/rule changes select tooling unit/integration
checks and generated-plan equivalence assertions, without application builds.
Do not substitute raw test commands or manually select category/component scripts.

Install the locked test-only Python dependencies once in the repository's isolated
verification environment (the scripts select it automatically):

```sh
python3.13 -m venv .venv-verification
.venv-verification/bin/python -m pip install --require-hashes -r requirements-verification.txt
```

The lock includes the existing JSON Schema contract-test dependency. Set `PYTHON`
only when intentionally using another interpreter with these locked packages.
Documentation-only light checks do not require the test packages.

The selector composes the existing category plans and runner, including all
selected compiler-backed, native, packaging and performance checks. Its text and
JSON output expose rule matches, selected checks, prerequisites and ordered stages
without creating evidence or builds. Existing full/light/category/component scripts
remain compatible internal building blocks. See [the testing strategy](<doc-12 - Testing-strategy.md>)
for the authoritative rule semantics and coverage requirements.

Report compilation, setup, execution and cleanup separately. The approximately
one-minute warm unit-execution goal is aspirational, never a reason to omit checks.
Native visual acceptance is distinct from headless UI tests, and offscreen GPU
throughput does not prove interactive/native displayed FPS. Formatting checks only
touched project-owned files and never rewrite vendored sources.

Full requires Apple Silicon macOS, native Metal/process-table access, Python 3.12+
(the local verification environment, falling back to `python3.13`), Rust with Clippy/rustfmt, Node 24+, JDK 25,
.NET SDK 11, Maven 3.9+/4, Graphviz and Xcode command-line/signing tools. Run it
outside the macOS sandbox. It also requires explicit inputs, configured once in
your shell (replace these example paths with your actual installations):

```sh
export CODECALLNAVIGATOR_TS_NODE_MODULES=/absolute/path/to/node_modules
export CODECALLNAVIGATOR_RECURRING_BILLING=/absolute/path/to/Recurring-billing
export CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE="/absolute/path/to/saved-application-data"
./verify-changes.sh --run
```

The modules directory must contain stable TypeScript 5.9.x or 6.0.x, React 19 and Next 16 fixture
dependencies. Preflight loads the TypeScript compiler with Node and checks that its version matches the
package manifest. The runner supplies the matching
`CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT` and engine binary paths; conflicting
TypeScript settings fail preflight. The private Recurring Billing acceptance
requires its Maven checkout and private-repository access; it is not silently
excluded. `CODECALLNAVIGATOR_TEST_MAVEN` can choose Maven for fixtures that support
that override; `mvn` must also be available on PATH for the other fixtures.

The profiling source is an application data directory with `catalog.sqlite3`,
saved navigation and an active published graph with relationships. You can also
pass `--profile-source /absolute/path`. The runner uses the SQLite backup API to
snapshot the catalog (including committed WAL data), copies the immutable graph
generation, and benchmarks only that copy. It never points benchmarks at the live
application store. Keep the selected source available through snapshot creation.
The standalone core storage-scale benchmark creates a synthetic database under its
category workspace and does not require a saved profiling source.

Each category cleans its owned Rust build outputs, fixtures/test stores and copied
profiling data before returning, including after failure, timeout or interruption.
Category builds live below their run evidence directory; logs, inventory selection,
acceptance evidence and timing/cleanup reports remain. Stop and reap owned processes
first. Ownership markers and native process inspection must prove inactivity before
deletion; uncertain or active artifacts are retained and fail cleanup. Abandoned
category workspaces are reclaimed only after proving ownership and inactivity.

`--keep-artifacts` explicitly retains diagnostic build state and reports retained
paths/sizes. Downloaded Cargo registry/Git caches remain; compiled category outputs
do not accumulate between categories. Report reclaimed/retained bytes. Rebuilding
after cleanup has a real cost and must not be hidden in execution-only timings.
Composite cleanup also preserves current packages/reports and one rollback per
current/legacy app name while pruning obsolete rotated packages. Cargo's own
age-based downloaded-cache management is disabled for verification children with
`CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never`; shared caches are not
manually purged.

The checkout lock prevents competing verification runs. Cleanup additionally
checks open-file usage with native `lsof` before removing artifact directories;
insufficient access or active users produce an explicit cleanup failure.

CodeScene uses the installed `cs-mcp` executable over stdio MCP, including its
normal authentication/license configuration. Select another executable path with
`--codescene-command`. Missing tools, unavailable checks, quality regressions and
incomplete results fail verification. Sonar remains a separate connected quality
gate if `.sonar/config.json` is configured; it is currently absent in this project.

A checkout lock prevents competing execution runs from sharing build/package
outputs. Execution writes fresh numbered stage logs, `results.json`,
`change-plan.json` and `summary.json` under `target/verification/`. Selected
categories record their own selection, cleanup and reclamation reports; full
verification additionally records its final root `cleanup.json`. Report-only and
clean-tree invocations create no evidence directory or builds.

Failed, timed-out and not-run stages remain distinct; policy exclusions are not
failed or passing tests. A failed/incomplete run never reports success.
The three real-window acceptance journeys check the active macOS console before
preparing fixtures or opening a window. A locked desktop or unavailable console
is recorded as not performed, with a reason in the gate result, stage log,
`results.json` and `summary.json`. Remaining checks and cleanup continue. If
nothing fails, a run with deferred checks reports `incomplete` and exits with
code 3. A stage or cleanup failure still reports `failed` and exits with code 1.
Headless and offscreen Metal checks have their usual prerequisites. The
standalone native pan profiler also checks the console before a new measurement.
`--timeout SECONDS` bounds each build/test/package stage (default 1800);
interruption/timeout stops the process group. If OS restrictions prevent descendant
cleanup, the script reports `cleanup-unavailable` and fails.
`--output /new/directory` retains evidence elsewhere; existing output directories
are rejected. `PYTHON=python3.12` chooses another supported interpreter.
`--base REF` controls committed changes plus staged, unstaged and nonignored
untracked files; the default base is HEAD. Clean-tree selection is empty.
Full verification is selected only by deterministic changed-file rules, never by
an agent choosing a scope flag.

See [TypeScript acceptance](<../history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-typescript-md>) and [Maven acceptance](<../history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-maven-md>) for the compatibility matrix. Headless Rune GPU screenshots and interaction checks are under
`evidence/ui`; `--scene fixture` is an explicit synthetic test scene, never normal
startup data. Real-project results and scale measurements are summarized in
[evidence/VALIDATION.md](<../history/doc-2 - Publication-and-release-verification-history.md#source-evidence-validation-md>). Live physical trackpad/window
validation remains outstanding: the execution session reports zero active displays.
