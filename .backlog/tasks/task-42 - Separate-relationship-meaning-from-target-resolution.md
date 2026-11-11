---
id: TASK-42
title: Separate relationship meaning from target resolution
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-11T14:38:00Z'
updated_date: '2026-11-11T15:00:00Z'
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
- [ ] #5 Compiler fixtures, regressions, UI checks, required Rust validation and Code Health reviews verify behavior with precise limitations.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Derive typed RelationshipKind (Calls, Function references, Framework/lifecycle, HTTP/navigation, Other) and TargetResolution (Directly resolved, Virtual/interface dispatch, Possible, Other/unspecified) in core, preserving raw analyzer facts. Apply boundary, reference, framework, invocation and fallback precedence; never upgrade unknown certainty. Cache once per GraphIndex generation. 2. Extract independent typed selections and isolated version-2 settings migration: preserve legacy code flags and reset all legacy relationship choices once; retain current empty selections and reject unsupported versions. Use one cached acceptance policy across traversal, paths, cycles, projections and hidden counts; keep classifications distinct during aggregation. 3. Use named UI descriptors, typed actions and shared presentation labels; show three groups above the drawable canvas so controls do not obstruct graph nodes or activate source previews. Update details, rendering and legend and remove obsolete overlay state, duplicate policies and unused projected raw fields. 4. Verify classification/precedence/unknown values, type-resolution combinations, legacy/current sessions, history, aggregation, fit and responsive accessibility; add assertions to real C#/Java/TypeScript compiler fixtures. 5. Review and refactor affected responsibilities, run scoped formatting, Rust checks, full compiler acceptance, release benchmarks, packaging, native visual checks and CodeScene reviews/safeguard. Record exact full-suite limitations without expanding into unrelated analyzer work.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer roles verified cache, migration, raw-artifact and aggregation invariants. Baseline migration regression failed as expected (missing version); grouped UI regression failed as expected (missing Code group). Implemented typed core classification cache, independent filter selections, isolated version-2 migration, semantic aggregation and grouped desktop controls/details/styles. Removed unused projected raw fields while preserving canonical raw facts. Core 55 tests and focused graph/projection/migration tests pass. Reviewer found no blocker and requested expanded evidence precedence/known-kind coverage, now added. CodeScene flagged a long classification test; extracted its static case table and restored score 10. Full desktop test run exposed overlay/hit-testing regressions from taller controls; fixing before final validation. Clippy currently passes. Sonar inactive because .sonar/config.json is absent.

Final nonignored desktop all-target run passes 335 tests (70 toolchain/performance cases ignored); core 55 tests pass. Cargo check and workspace Clippy -D warnings pass. UI94 and edge/render16 pass after normal-flow layout fix and canvas-local popup fixture placement. Native Metal dark1420x900 and light1050x680 captures inspected; supported minimum confirmed in AppSpec. Packaging builds/signs successfully. Independent reviews found no blockers. CodeScene safeguard passed34/34 eligible files, no regressions. Full ignored compiler suite and appropriate release benchmarks still running; evidence is evidence/ui/relationship-filters-2026-11-11/README.md.

Validation finished: final root all-target tests335 passed/70 ignored; core55 passed; check/clippy/format/diff checks pass. Full outside-sandbox include-ignored run526 passed/7 failed. Targeted reruns resolve six: corrected popup scenario passes final UI run; private duplicate test passes152.92s; compiled-private metadata passes69.25s; all3 release CPU/GPU benchmarks pass9.49s with isolated saved cache. Only TypeScript source-alias artifact equality remains: fails rebuilt current engine and pre-change packaged engine at same assertion; baseline is point-in-time package, not exact HEAD. Paths changed by this task do not include analyzer/artifact canonicalization. No unrelated fix or follow-up task created. TASK42 stays In Progress for final validation sign-off; feature implementation and reviewed related checks complete. Evidence and native captures: evidence/ui/relationship-filters-2026-11-11/README.md. No commit or push.
<!-- SECTION:NOTES:END -->
