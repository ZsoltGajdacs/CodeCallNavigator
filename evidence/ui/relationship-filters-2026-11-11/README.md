# Relationship meaning and target resolution

TASK-42 implements the approved independent semantic dimensions. Raw analyzer
facts and canonical graph artifacts remain unchanged. Classifications are derived
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

## Full-suite limitations and targeted reruns

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
- The TypeScript source-alias artifact-equality test still fails with a freshly
  rebuilt current engine. It also fails with the pre-change packaged engine
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

The remaining full-suite failure is the independently reproduced source-alias
artifact discrepancy. TASK-42 remains In Progress for final validation sign-off;
acceptance criteria 1–4 are verified. Implementation changes are uncommitted.

Detailed execution logs remain in `/tmp/codegraph-filter-full-unsandboxed.log`,
`/tmp/codegraph-filter-private-rerun.log`,
`/tmp/codegraph-filter-private-compiled-rerun.log`,
`/tmp/codegraph-filter-release-benchmarks.log`,
`/tmp/codegraph-filter-source-path-retry.log`, and
`/tmp/codegraph-filter-source-path-baseline.log`.

