---
id: TASK-77
title: Support TypeScript 5.9.x alongside 6.0.x
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-17T08:03:00Z'
updated_date: '2026-11-17T10:36:00Z'
labels: []
dependencies: []
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The desktop rejects project TypeScript 5.9.3 because compiler loading and verification only accept 6.0.x. Add the approved compatibility range with shared policy and real compiler regression coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Stable project-installed 5.9.x and 6.0.x compilers are accepted; other families and prereleases fail with actionable diagnostics.
- [x] #2 Adapter and verification use one compatibility policy; policy and compiler changes invalidate freshness.
- [ ] #3 Real compiler integration coverage passes on 5.9.3 and 6.0.3 without emitted build outputs or graph contract changes.
- [x] #4 Documentation, verification inventory and Code Health checks reflect the change.
- [x] #5 The four observed UI verification blockers pass: freshness controls remain reachable and flow canvases retain usable height at the minimum viewport.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Share a packaged compiler compatibility policy and extract pure validation. 2. Update prerequisite checks and diagnostics; preserve graph schema and adapter 7. 3. Add policy, metadata and compiler regression tests and inventory entries. 4. Update canonical docs. 5. Run complete deterministic verification for isolated 5.9.3 and 6.0.3 dependencies and Code Health review.

6. User-approved extension: reveal offscreen Activity freshness actions before UI test clicks; bound and scroll flow notices to reserve chart space at minimum viewport; strengthen existing layout assertions and rerun the complete selector on 5.9.3 and 6.0.3.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared shipped policy, pure JS/Python eligibility helpers, input observation/fingerprinting and diagnostic updates. Added version-policy unit cases, shared preflight acceptance, metadata checks, worker policy mutation rejection and packaged compiler/policy freshness coverage. Code Health review scored 10.0 on all changed source; safeguard passed 24 eligible files. Initial full runs stopped at preflight for missing configured private checkout/profile source; user supplied checkout path and rerun now progresses with real 5.9.3. No standalone tests substituted for deterministic selector.

Read-only review found no production defects; added explicit compiler-entry invalidation coverage using a private compiler copy and fixed worker cleanup ownership. Full preflight now succeeds using the user-supplied Recurring-billing path and saved app profile. First runtime attempt stopped at two existing Clippy errors in core observation tests; corrected needless borrow and octal permission notation without behavior changes, then reran the complete selector plan. Evidence remains under target/verification/.

TypeScript 5.9.3 real pipeline integration passed all 10 selected tests in tests/typescript_pipeline.rs, including exact compiler metadata, project references, packaged resources, compiler/policy freshness invalidation, failure preservation and worker bounds. Node/Python/Rust unit stages also passed. Full verification continues through remaining integration/acceptance/performance stages; 6.0.3 rerun remains pending.

Full 5.9.3 run changes-e9iwi35e stopped at integration-test-rust: two freshness UI journeys timed out and C# flow/expansion minimum-window checks found zero-height flow-canvas. Later Node integration, acceptance and performance stages were not run; cleanup completed. Static inspection points to offscreen Activity action clicks and unbounded flow notices, not compiler eligibility, but no baseline run was performed. Full 6.0.3 comparison run changes-cyk9jc5j is in progress. Asked user whether to include UI blocker fixes or retain TypeScript scope. Portable artifact provenance retains its existing explicit-root/unmapped-input contract; current compiler/policy invalidation test directly proves runtime fingerprints.

Final verification checkpoint: both real TypeScript 5.9.3 (changes-e9iwi35e) and 6.0.3 (changes-cyk9jc5j) passed static checks, Clippy, all selected unit stages and the 10 real TypeScript pipeline tests, including compiler/policy fingerprint invalidation. Both full runs failed the same four UI integrations: csharp_flows::real_csharp_controls_keep_exact_source_cleanup_targets_and_history; cli_freshness::actual_cli_automatic_refresh_requires_opt_in_and_preserves_loaded_context; cli_freshness::actual_cli_freshness_reload_preserves_loaded_binding_camera_and_discards_obsolete_jobs; flow_expansion::expansion_mode_is_explicit_and_collapse_all_restores_the_ordinary_session. Each run marked 32 later stages not run, including Node integration, acceptance and performance. New worker policy-mutation test has therefore not executed. No baseline run was performed, so these are not claimed as preexisting. Full gate and AC3 remain pending; task stays In Progress. User scope question remains pending; no UI production changes made. CodeScene reviewed all 25 modified source files at 10.0; final safeguard passed 25 eligible files, no regressions. Sonar is inactive because this project has no .sonar/config.json. All selected cleanup stages passed. Timing seconds: 5.9.3 setup/static 9.84, compilation/checking 360.14, executed tests 656.81, cleanup 7.04; 6.0.3 setup/static 9.73, compilation/checking 293.82, executed tests 587.83, cleanup 6.53. Reusable isolated TypeScript 5.9.3 toolchain remains in target/verification/typescript-compatibility/5.9.3 (23 MB) and npm download cache in target/verification/npm-cache (19 MB) for reruns; category builds and temporary fixtures were cleaned. Changes remain uncommitted and uninstalled.

User explicitly approved including fixes for all four UI verification blockers. No exception or waiver requested; complete verification remains required.

Implemented the approved UI extension: the single-flow notice is naturally sized up to 180px and vertically scrollable; the UI driver reveals freshness actions inside Activity or flow notice scroll containers before clicking, then asserts full visibility. Extended the existing real Rune minimum-window regression with long diagnostic content in both themes, requiring at least 150px visible chart height and reachability of Copy details. Code Health initially flagged test nesting; extracted named layout/assertion operations and re-reviewed at 10.0. Full 5.9.3 verification with UI fixes is running; no raw test commands substituted.

First UI-fix run changes-w6acj_z7 reached the strengthened layout regression and failed in its setup because State::fixture has no generation. Corrected the presentation-only setup to assign an explicit matching 64-character generation to state, query descriptor and freshness observation. No production workaround or assertion weakening. Complete 5.9.3 selector rerun is active with C# and freshness native capture paths under target/verification/typescript-compatibility/ui-captures/5.9.3. Existing regular integration group otherwise passed 238 tests before the fixture failure; original ignored four cases were not reached in that run.

Run changes-_dle9yev passed the corrected long-notice regression and three original failures (automatic refresh, C# minimum layout, expansion restore). Remaining reload UI failed because its action was fully clipped after the one-shot reveal: rect [318,861,157.39,24], visible height 0. Rune reveal requests stop tracking after current layout settles; freshness can update the body afterward. Revised driver ownership: maintain the reveal during the existing 60-frame settling interval, target the active element full path, use unique Activity ancestor scope, and log all relevant scroll geometry on failure. No timeout increase, assertion weakening or upstream changes. New complete 5.9.3 run is active. Reviewed native light minimum C# capture: chart and source now retain usable space at 1050x680; capture under target/verification/typescript-compatibility/ui-captures/5.9.3/csharp/light-minimum.png.

The active 5.9.3 run changes-80jtfl_f passed all four original UI failures, including the reload/context/camera/obsolete-job journey, plus the long-notice minimum-window regression. The revised reveal follows the active full-path control while popup layout changes; full visibility assertions remain intact. No further source changes planned while the complete selected plan continues into later stages. Both light and dark 1050x680 native C# captures were inspected; chart and source remain visible below the bounded notice.

Run changes-80jtfl_f passed all Rust integrations, all four UI blockers, and Python integration (274 tests), then reached Node integration for the first time. Worker policy-mutation rejection passed. Two Node failures remained: paths.test expected only canonical missing path although production deliberately retains lexical aliases too; signature.test found absolute import-type module paths in signatureToString (same fallback exists in both compiler versions). Updated the path assertion to require exactly physical and lexical paths. Added cohesive compiler-token signature normalization for complete absolute import module arguments, preserving authored quoted/template literals, formatting and truncated text; canonical source base and directory aliases reuse paths.js. Added linked-source, nested-template and truncation integration cases (inventory1591). Complete 5.9.3 rerun now active. No raw test commands or weakened assertions.

Latest 5.9.3 run changes-tq3r1g_4 passed all Rust integration, Python integration (274 tests), Node integration including all signature/literal/template/linked-source/truncation cases, worker policy-mutation rejection, and the C# flow acceptance gate. All four UI blocker journeys pass. Acceptance packaging is now running. Canonical adapter doc updated with signature-normalizer ownership; no public schema/version changes.
<!-- SECTION:NOTES:END -->
