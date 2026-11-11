# Relationship meaning and target resolution

TASK-42 implements the approved independent semantic dimensions. The filter implementation preserves raw analyzer relationship facts and semantic
graph content; the approved follow-up below improves canonical input provenance. Classifications are derived
once for each indexed generation, and shared by filtering, aggregation, drawing
and details. Session filter version 2 resets legacy relationship choices once while
preserving code flags; current empty selections persist.

The panel occupies layout space above the drawable canvas. An overlay prototype
caused underlying graph hover previews to obscure filter controls; normal flow
removes that overlap and keeps camera fitting based on the actual canvas size.
Unused overlay-inset state, projected raw fields, and duplicate string-based
classification helpers were removed. Filter descriptors, session migration and
classification rules each have a focused owning module.

## Visual checks

Native Metal captures were inspected with current production rendering:

- [Dark, 1420×900](dark-1420.png)
- [Light, supported minimum 1050×680](light-1050.png)

Full labels, grouping and wrapping are readable. The supported minimum is declared
in `src/main.rs`; smaller headless captures do not represent supported window sizes.
CPU UI tests cover both themes, panes open/closed, history, migration, empty
selections, accessible help, fit below the panel, and normal graph interaction.

## Verification

- Baseline migration regression failed as expected before implementation: version
  was absent. Baseline grouped UI regression failed on the missing Code heading.
- Core tests: 55 passed.
- Focused graph/projection/filter tests pass, including evidence precedence,
  unknown analyzer values, separate reference/virtual selection, cycle/path/count
  consistency, and equal raw call/possible edges with different semantic meaning.
- Desktop check, all nonignored target tests, and workspace Clippy with warnings
  denied pass. All 94 nonignored UI tests and 16 edge/render tests pass.
- `python3.13 tools/package.py` built and ad hoc signed the desktop app and engine.
- Independent core and desktop review found no correctness blockers; requested
  evidence-precedence and complete kind coverage was added.
- CodeScene reviews report 10.0 with no findings on modified source after focused
  test/module extraction. Sonar is inactive: `.sonar/config.json` is absent.

## Initial full-suite failures and targeted reruns

The exact debug include-ignored aggregate is not green. The outside-sandbox
`cargo test --workspace --all-targets --no-fail-fast -- --include-ignored --test-threads=1`
run reported 526 passed and 7 failed. Those failures comprise three benchmarks
that explicitly require release mode, one source-popup fixture placement already
corrected and covered by the final passing desktop run, two private acceptance
cases requiring explicit local fixture paths, and one TypeScript alias discrepancy.
The initial sandbox attempt also encountered restricted dependency restores and
process-group cleanup; the cancellation check passed with the required access.

- The private duplicate-declarations test passes against an existing isolated
  temporary reactor copy (152.92 seconds), without touching the original checkout.
- Before the approved follow-up, the TypeScript source-alias artifact-equality
  test failed with a freshly rebuilt engine. It also fails with the pre-change packaged engine
  `CodeCallNavigator.previous-20261111-120921.app` at the same assertion. This is a
  point-in-time packaged baseline comparison, not an exact-HEAD baseline proof.
  Differences concern dependency/missing-probe inputs and derived diagnostics;
  these analyzer/artifact paths were not changed for TASK-42.
- All public Java, C# and TypeScript compiler acceptance cases other than that
  alias discrepancy passed, including the new method-reference, virtual-dispatch,
  Java-lambda and React-framework classification checks.

- Compiled-private metadata acceptance passes with the existing isolated reactor
  metadata (69.25 seconds).
- All three release benchmarks pass with Metal and a copied saved cache (9.49
  seconds total). Dense CPU frame preparation p95 is 3.250–4.101 ms; saved CPU
  preparation p95 is 5.421 ms at scale 1 and 5.565 ms at scale 2; GPU-complete
  saved frame p95 is 11.290 ms at scale 2. The original workspace was untouched.

At that checkpoint, the remaining full-suite failure was the independently
reproduced source-alias artifact discrepancy. TASK-42 remained In Progress for
validation sign-off. The filter implementation was subsequently committed as
`31387fb`; the approved follow-up below resolves that discrepancy.

Detailed execution logs remain in `/tmp/codegraph-filter-full-unsandboxed.log`,
`/tmp/codegraph-filter-private-rerun.log`,
`/tmp/codegraph-filter-private-compiled-rerun.log`,
`/tmp/codegraph-filter-release-benchmarks.log`,
`/tmp/codegraph-filter-source-path-retry.log`, and
`/tmp/codegraph-filter-source-path-baseline.log`.


## Investigation after commit 31387fb

The filter implementation was committed locally as `31387fb` on
`codex/standalone-graph-engine`; nothing was pushed.

A retained reproduction ran the same fixture four times: alias spelling,
canonical spelling, canonical repeat, and alias repeat. Results:

- The complete artifact structures are equal after removing only `graph.analysis`.
  All declarations, identities, relationships, evidence, source flows,
  architecture, boundaries and diagnostics match.
- Each spelling produces byte-identical artifacts when repeated. This is not
  nondeterministic graph output.
- The test rewrites the actual `tsconfig.json` source-path strings between
  spellings. Its observed configuration content hash therefore correctly differs.
  Comparing the whole artifact across these different input bytes is invalid.
- Canonical spelling retains ten additional portable missing-file observations.
  The adapter normalizes paths with `realpathSync` only when the entire path exists;
  absent probes below symlinked directories retain alias prefixes. Logical-root
  mapping cannot map those prefixes, and records them as `unmapped_input`.
  The alias run has 262 unmapped observations versus 251 for canonical spelling.
  Both independently verify as `unverifiable` with reason `unmapped_input`,
  so the verifier does not claim complete input verification.

The appropriate regression contract is semantic graph equality across source
spellings, byte-for-byte determinism across identical repeated inputs, and
independent validation of captured provenance/configuration hashes. Input hashes
must not be removed or normalized away to make the existing assertion pass.
Missing-path alias normalization is a separate analyzer/provenance concern.

Independent investigation review confirmed these results. At that stage TASK-42
had no unfinished filter implementation; only validation/sign-off was pending.
It remained In Progress until the incorrect test expectation was corrected or
explicitly accepted. The investigation itself changed no analyzer or test code;
the subsequently approved implementation is described below.

The reproducible driver is `/tmp/codegraph-alias-investigation.py`; its retained
fixture path is recorded in `/tmp/codegraph-alias-investigation.log`. Four artifact
snapshots, exact config spellings, raw metadata and per-spelling verification
reports remain in that temporary fixture directory. No private source payloads
were added to the repository.


## Approved follow-up fixes

The user approved both recommended fixes after the investigation.

- `02010eb` normalizes missing TypeScript input paths through the nearest existing
  physical ancestor. Filesystem mappings are not cached; only missing-path errors
  are retried, and other filesystem errors propagate.
- Original compiler probe arguments and read closures remain intact. A separate
  observation of each original-to-physical input mapping detects symlink
  retargeting even when both destinations still lack the probed file.
- Adapter provenance version is now 6. No wire-format or relationship-inference
  change is required; rebuilt engine and desktop packages include the new scripts.
- The real-engine source-alias acceptance test compares full semantic artifact
  structures across spellings, verifies exact BLAKE3 hashes of actual configuration
  bytes, compares every other provenance field/input, and checks full-byte repeats
  separately for each unchanged spelling. The comparison orders cloned input rows
  by path after checking the configuration hash, because production JSON ordering
  includes that deliberately changed hash. Production artifact ordering is intact.

The eight real-filesystem/compiler path regressions had five expected baseline
failures before the fix and all pass afterward. The strengthened Rust acceptance
first reproduced missing portable input coverage, then passed after normalization.
All 112 JavaScript tests pass, including retained worker/source/probe integrity and
native binding tests. Independent review found no actionable issues; all six
changed source files score CodeScene 10.0 and the safeguard passes. Cargo check,
strict workspace Clippy, and rebuilt/signed packaging pass.

The full release desktop target passes all 188 tests, including all three CPU/GPU
benchmarks. A parallel debug run during packaging/JS load hit eight existing
2-second flow readiness timeouts. The serial debug all-target repeat passes
335 tests with 70 toolchain/performance cases ignored, without changing those
readiness tests. Complete release workspace validation passes as recorded below.


The alias-dependent missing-input gap is resolved. External dependencies outside
all declared logical roots still produce explicit `unmapped_input` limitations;
the fix preserves those limitations rather than claiming complete verification.
The strengthened acceptance test requires equivalent limitation counts across
spellings as well as equivalent normalized inputs.


## Final sign-off

TASK-42 is complete. The corrected source-alias acceptance no longer conflates
changed configuration provenance with semantic graph equality, and missing input
probes retain equivalent physical/logical coverage across source spellings.

- Full release workspace: **533 passed, 0 failed, 0 ignored**, across 46 outer
  targets (excluding a duplicate result emitted by a nested subprocess test).
- All compiler fixtures, both private reactor/compiled-metadata checks, the
  strengthened source-alias acceptance, process/protocol tests, and all three
  CPU/GPU benchmarks pass in that single supported-profile run.
- Serial debug desktop all-targets: **335 passed, 70 ignored**. Those ignored
  cases are covered by the full release run.
- JavaScript compiler/adapter suite: **112 passed**, including all eight focused
  filesystem/compiler path tests.
- Cargo check, strict workspace Clippy, scoped formatting, CodeScene reviews and
  safeguard, independent review, and rebuilt/signed packaging pass. Sonar remains
  intentionally inactive because the project has no `.sonar/config.json`.

Full workspace command:

```sh
cargo test --release --workspace --all-targets -- --include-ignored --test-threads=1
```

The run uses the actual toolchains, a copied profiling cache, explicit isolated
private reactor and compiled-metadata paths, and the freshly built release engine.
The private reactor's frontend dependency metadata retries caused a long delay
but its complete Maven build succeeded; no frontend-skip profile was used.

Final logs: `/tmp/codegraph-alias-release-full.log`,
`/tmp/codegraph-alias-regular-serial.log`, `/tmp/codegraph-alias-all-js.log`,
`/tmp/codegraph-alias-fixed-test.log`, `/tmp/codegraph-alias-clippy.log`, and
`/tmp/codegraph-alias-package.log`. No private graph or source payloads are checked
in. Nothing has been pushed.
