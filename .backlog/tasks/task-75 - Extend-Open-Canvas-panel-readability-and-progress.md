---
id: TASK-75
title: Extend Open Canvas panel readability and progress
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T21:33:00Z'
updated_date: '2026-11-16T22:13:00Z'
labels: []
dependencies: []
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement approved mockup 1 for readable native panels, title-bar progress, continuously animated spinner and centered canvas controls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Panels match mockup 1 with readable bounded layout
- [x] #2 Progress is visible alongside warnings
- [x] #3 Spinner advances and honors reduced motion
- [x] #4 Controls center without overlap
- [x] #5 Required verification and CodeScene pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Implement mockup 1 panels and shared title-bar phase. Investigate the reported spinner freeze and verify rotation across progress updates, idle and reduced motion; patch only a demonstrated cause. Center responsive graph controls. Run the complete deterministic verification plan, native visual QA and CodeScene; update doc-34.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented selected mockup 1 surfaces, title-bar phase independent from warning emphasis, custom width-bounded activity dialog, four-row legend, and centered/stacked graph controls. Independent review corrected recent-project row height, freshness typography, freshness-check phase and activity expanded accessibility state. Added real Rune layout/paint and Metal capture tests; deterministic verification running. Existing Loader already declares rotation; freeze cause remains under investigation rather than assumed fixed.

Validation: 153 UI unit tests passed. Timed Rune paint regression proved the existing busy ring rotates across frames and progress updates, respects reduced motion, and removes its busy loop when idle; the originally reported physical-window freeze was not reproduced, so no speculative vendor patch was made. The new layout test exposed a real 12-point wide-canvas centering offset from combining full width with inset pinning; removed the conflicting width. Subminimum 600-point component stress excludes simultaneous docked panes because the actual native minimum is 1050x680. Independent final review has no blocking findings. Complete deterministic verification rerunning after correction.

Count correction: the unit stage contains 152 desktop UI unit tests plus 68 core/engine unit tests (220 total), all passing. Final review confirmed all earlier findings resolved and the wide parent now uses its pin-constrained width.

Final source verification: all 42 deterministic stages passed; 220 unit and 262 integration registrations plus all 3 packaged-app acceptance gates passed. Evidence: target/verification/changes-xe7_urnw/. Final native captures and visual QA: target/verification/panel-captures-final/ (design-qa.md). Timings: compile=189.384s, setup=15.777s, test=187.300s, cleanup=6.897s, other=98.338s. All owned cleanup passed. CodeScene reviews scored 10.0 and the pre-commit safeguard passed across 12 modified Rust files. Sonar is inactive because .sonar/config.json is absent. Visual QA corrected disabled-project contrast by disabling the native element after normal label styling, preserving input restrictions without fading project identity. Spinner paint and rendered-frame orientations advance; the original physical-window freeze was not reproduced, and offscreen captures do not certify physical-window scheduling.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented approved mockup 1 with readable 400/480-point project/activity panels, separated title-bar progress, labeled graph counts/legend/freshness, readable disabled project identities and canvas-centered responsive controls. Added native layout, scrolling/focus and timed animation coverage. All selected verification, cleanup, visual QA and CodeScene passed. Original freeze not reproduced; existing Loader animation verified without speculative upstream edits. Evidence and stage timings are in implementation notes.
<!-- SECTION:FINAL_SUMMARY:END -->
