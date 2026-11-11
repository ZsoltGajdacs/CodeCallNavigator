---
id: TASK-42
title: Separate relationship meaning from target resolution
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T14:38:00Z'
updated_date: '2026-11-11T17:52:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The existing filter row mixes code inclusion, relationship meaning and uncertainty. Implement the approved independent classification model with maintainable shared semantics and presentation-only grouping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Core classifies supported relationships into five kinds and four resolution categories without changing raw analyzer facts or canonical artifacts.
- [x] #2 Independent typed selections govern traversal, paths, cycles, projections, aggregation and hidden counts.
- [x] #3 Grouped responsive controls, help, legend, rendering and details consume shared classification; duplicate policies and positional tuples are removed.
- [x] #4 Legacy views preserve code flags and reset relationship selections once; new selections survive serialization, history and reload.
- [x] #5 Compiler fixtures, regressions, UI checks, required Rust validation and Code Health reviews verify behavior with precise limitations.
- [x] #6 Missing TypeScript input probes below symlinked parents normalize consistently while original compiler observations still detect input changes.
- [x] #7 Alias acceptance compares semantic graphs and normalized input coverage, verifies actual configuration hashes, and checks byte determinism for repeated unchanged inputs.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Derive typed RelationshipKind (Calls, Function references, Framework/lifecycle, HTTP/navigation, Other) and TargetResolution (Directly resolved, Virtual/interface dispatch, Possible, Other/unspecified) in core, preserving raw analyzer facts. Apply boundary, reference, framework, invocation and fallback precedence; never upgrade unknown certainty. Cache once per GraphIndex generation. 2. Extract independent typed selections and isolated version-2 settings migration: preserve legacy code flags and reset all legacy relationship choices once; retain current empty selections and reject unsupported versions. Use one cached acceptance policy across traversal, paths, cycles, projections and hidden counts; keep classifications distinct during aggregation. 3. Use named UI descriptors, typed actions and shared presentation labels; show three groups above the drawable canvas so controls do not obstruct graph nodes or activate source previews. Update details, rendering and legend and remove obsolete overlay state, duplicate policies and unused projected raw fields. 4. Verify classification/precedence/unknown values, type-resolution combinations, legacy/current sessions, history, aggregation, fit and responsive accessibility; add assertions to real C#/Java/TypeScript compiler fixtures. 5. Review and refactor affected responsibilities, run scoped formatting, Rust checks, full compiler acceptance, release benchmarks, packaging, native visual checks and CodeScene reviews/safeguard. Record exact full-suite limitations without expanding into unrelated analyzer work.

6. User-approved validation follow-up: normalize missing observed paths through nearest existing ancestor, preserve original observation checks, cover missing paths and symlink retargeting, and update TypeScript adapter provenance version as needed. 7. Repair source-alias acceptance to compare semantic graph, normalized analysis inputs except actual config hash, repeated full bytes and exact config hashes. 8. Run focused compiler/adapter suites, required Rust and packaging checks, independent review and CodeScene; finalize TASK42 only after all remaining applicable gates pass.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer roles verified cache, migration, raw-artifact and aggregation invariants. Baseline migration regression failed as expected (missing version); grouped UI regression failed as expected (missing Code group). Implemented typed core classification cache, independent filter selections, isolated version-2 migration, semantic aggregation and grouped desktop controls/details/styles. Removed unused projected raw fields while preserving canonical raw facts. Core 55 tests and focused graph/projection/migration tests pass. Reviewer found no blocker and requested expanded evidence precedence/known-kind coverage, now added. CodeScene flagged a long classification test; extracted its static case table and restored score 10. Full desktop test run exposed overlay/hit-testing regressions from taller controls; fixing before final validation. Clippy currently passes. Sonar inactive because .sonar/config.json is absent.

Final nonignored desktop all-target run passes 335 tests (70 toolchain/performance cases ignored); core 55 tests pass. Cargo check and workspace Clippy -D warnings pass. UI94 and edge/render16 pass after normal-flow layout fix and canvas-local popup fixture placement. Native Metal dark1420x900 and light1050x680 captures inspected; supported minimum confirmed in AppSpec. Packaging builds/signs successfully. Independent reviews found no blockers. CodeScene safeguard passed34/34 eligible files, no regressions. Full ignored compiler suite and appropriate release benchmarks still running; evidence is evidence/ui/relationship-filters-2026-11-11/README.md.

Validation finished: final root all-target tests335 passed/70 ignored; core55 passed; check/clippy/format/diff checks pass. Full outside-sandbox include-ignored run526 passed/7 failed. Targeted reruns resolve six: corrected popup scenario passes final UI run; private duplicate test passes152.92s; compiled-private metadata passes69.25s; all3 release CPU/GPU benchmarks pass9.49s with isolated saved cache. Only TypeScript source-alias artifact equality remains: fails rebuilt current engine and pre-change packaged engine at same assertion; baseline is point-in-time package, not exact HEAD. Paths changed by this task do not include analyzer/artifact canonicalization. No unrelated fix or follow-up task created. TASK42 stays In Progress for final validation sign-off; feature implementation and reviewed related checks complete. Evidence and native captures: evidence/ui/relationship-filters-2026-11-11/README.md. No commit or push.

User requested commit then investigation. Committed filter implementation as31387fb on codex/standalone-graph-engine; nothing pushed. Four retained fixture runs (alias, canonical, canonical repeat, alias repeat) prove complete artifacts equal after removing only graph.analysis, and each spelling repeats byte-identically. Existing test rewrites tsconfig.json literal source paths, so correct captured configuration hash differs: whole-artifact cross-spelling byte equality is an invalid assertion. Separate missing-probe normalization issue confirmed: paths.js realpaths only existing full paths; alias-prefixed absent probes cannot map to canonical roots. Canonical retains10 additional portable missing observations, alias unmapped262 versus canonical251. Both verify as unverifiable/unmapped_input, not falsely verified. Independent review confirms diagnosis. Correct test should compare semantics across aliases, full bytes across unchanged repeats, and provenance/config hashes independently. Do not weaken actual input hashes. TASK42 implementation complete, AC5 still pending test correction or explicit acceptance; no analyzer/test changes or new follow-up task made during investigation. Details appended to evidence/ui/relationship-filters-2026-11-11/README.md.

User explicitly approved both recommended fixes and resolving TASK42. Existing uncommitted changes at start are investigation notes in this task and its evidence README; preserving them. Filter implementation remains committed as31387fb. No push authorized or performed.

Approved fixes implemented and independently reviewed. Normalization commit02010eb resolves nearest existing ancestors, observes original-to-physical path mappings separately to detect symlink retargeting with absent files, preserves compiler probe closures, and reports adapter6. Eight focused filesystem/compiler path regressions reproduced5 expected baseline failures then all passed; all112 JavaScript tests pass. Revised Rust acceptance initially reproduced missing@repo/src/app/package.json; after fix, coverage and semantic graphs match and repeat bytes are stable. Comparison now sorts cloned input rows by path after checking/removing only actual config hash, because production canonical row order includes that deliberately changed hash. Focused real-engine acceptance passes; all6 affected source files CodeScene10 and safeguard passes. Packaging rebuilt/signed successfully; final check/Clippy pass. Parallel debug suite during packaging/JS load hit8 existing2s flow readiness timeouts; full proper-profile release workspace run is now serial with all private prerequisites and copied profiling data.

Strengthened alias acceptance passes with actual config hashes and all remaining provenance preserved; expected config-driven row reordering is normalized only in test comparison. Full JavaScript suite112/112, focused8path tests and independent review pass. Final serial debug all-target suite335 passed/70 ignored, resolving prior parallel-load flow readiness timeouts without changing readiness tests. Full release workspace run uses supplied private metadata/reactor and copied profiling cache, with no failures through build/inference fixtures; remaining long-running fixtures still executing.

Final full validation is green: cargo test --release --workspace --all-targets -- --include-ignored --test-threads=1 exited0 with533 passed/0 failed/0 ignored across46 outer targets, including188 desktop tests, all3 CPU/GPU benchmarks, all language fixtures, repaired alias acceptance, private reactor duplicates and private compiled metadata. Full copied private reactor Maven build succeeded; nonfatal frontend npm metadata/DNS retries explain its long duration, and no frontend-skip profile was used. Serial debug335 passed/70 ignored, JS112 passed, check/Clippy/scoped formatting/diff checks pass. Independent review approved both fixes. All6 modified source reviews score10; final safeguard passed1 eligible remaining file. Rebuilt/signed packaging passed. Sonar intentionally inactive (no project config). No remaining unmet AC/DoD or unaccepted quality exceptions. Evidence and full logs linked in evidence/ui/relationship-filters-2026-11-11/README.md. No push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented shared relationship classifications and independent grouped filters with consistent traversal, projection, rendering, details and saved-view migration (31387fb). Approved follow-up normalization (02010eb) resolves missing inputs through symlinked ancestors while preserving compiler observations and detecting mapping changes; adapter provenance is6. Corrected source-alias acceptance verifies semantic graph equality, exact config hashes, equivalent remaining provenance, and full-byte repeats without weakening production validation. All533 release workspace tests pass with none ignored;112 JS and335 serial debug tests pass, alongside strict Clippy, CodeScene, review and signed packaging. TASK42 complete; changes remain local.
<!-- SECTION:FINAL_SUMMARY:END -->
