---
id: TASK-46
title: Move build selection into an anchored modal
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T09:57:00Z'
updated_date: '2026-11-12T11:37:00Z'
labels: []
dependencies: []
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The inline build list consumes sidebar space and displays long paths. Move selection beside Find builds and make selected builds visible on hover.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Find builds opens a scrollable anchored modal and rescans; selection applies immediately with keyboard, Escape, outside dismissal and focus return.
- [x] #2 Compact labels use readable declared project names and safe fallbacks consistently across discovery paths and hover summaries.
- [x] #3 Rescans preserve selections, new targets use defaults, and stale results cannot replace the active folder.
- [x] #4 Discovery lifecycle, picker presentation and shared naming have cohesive ownership; required tests and CodeScene checks are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Separate metadata naming and cover declared names/fallbacks. 2. Extract discovery lifecycle and selection reconciliation. 3. Add anchored modal, hover summary and overlay coordination. 4. Add UI regressions, run required checks and CodeScene, record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented picker/discovery separation and compact metadata names. Static reviewer identified and verified fixes for old targets surviving folder changes and stale discovery warnings. First focused discovery tests and seven real Rune UI picker tests pass. Full validation exposed test assumptions tied to the old inline height/session reset; updating these. CodeScene desktop source reviews are 10/10. Sonar configuration absent, so Sonar workflow is inactive.

Final implementation verified by 8 real Rune UI tests, 4 discovery lifecycle regressions, 60 core tests and regular full workspace run (494 passed, 84 ignored); Clippy, checks, 106 Python tests and local release packaging/signature verification passed. Native dark/light screenshots and detailed commands/results: evidence/ui/build-picker-2026-11-12/README.md. All modified source reviews scored 10.0; final CodeScene safeguard passed for 15 eligible files. Independent reviewer found no remaining blockers after fixes. Required include-ignored gate remains failing: 3 release-only benchmark prerequisites and sandbox forced-cancellation child cleanup assertion. Expanded no-fail-fast attempt also reported 5 ASP.NET failures then stalled in dotnet restore; terminated that attempt and its child restore. Task remains In Progress because full ignored-test validation has no accepted exception. No commit/push/PR created.

The final native full verification from TASK-47 resolves the previous full-suite limitation: all578 release workspace tests (0 ignored), compiler acceptance, all canvas benchmarks, packaging and CodeScene now pass with no waivers. Build picker UI regressions and native screenshots remain valid. Subsequent verification record linked from original evidence/ui/build-picker-2026-11-12/README.md. Local package contains the picker; source changes remain uncommitted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Moved build selection into a right-anchored scrollable modal with immediate checkbox updates, compact declared project names, selected-build hover summary and selection-preserving rescans. Separated picker presentation, discovery lifecycle and shared naming. Native UI screenshots/regressions and the subsequent full verification pass (578 release tests, compiler/benchmark/package/CodeScene checks); no remaining validation exception. No commit/push.
<!-- SECTION:FINAL_SUMMARY:END -->
