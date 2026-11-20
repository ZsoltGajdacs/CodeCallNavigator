---
id: TASK-100
title: Fix the desktop Compare crash
status: Done
assignee:
  - '@codex'
created_date: '2026-11-20T07:13:00Z'
updated_date: '2026-11-20T07:35:00Z'
labels: []
dependencies: []
type: bug
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rune panics when Compare mounts a virtual list whose column lacks a GrowthSpec. The affected views are changed-symbol impact, graph differences, and graph change JSON.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A populated changed-symbol impact preview renders its symbol and caller rows.
- [x] #2 A populated graph differences list renders and its first change opens and closes JSON detail without a panic.
- [x] #3 The real-engine desktop acceptance test renders all three affected views through Rune.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extend the existing snapshot acceptance journey to render the impact and graph views, open and close JSON detail, and reproduce the panic through the selector.
2. Keep each view’s rows builder local and add GrowthSpec to its virtualized column. Preserve scrolling and row bindings.
3. Review Code Health, run the complete selector and smoke-test the native views when the console is available.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added real-engine Rune rendering coverage for impact results, graph changes, and JSON detail. The test reproduced the exact reported Rune panic before the fix in target/verification/compare-growth-before/28-acceptance-test-rust.log. Configured all three virtualized columns and kept each row builder local. CodeScene findings led to extracting the JSON toolbar and simplifying test assertions. All three Rust files score 10.0 after review. Development selector passed all five stages.

Final selector passed 35 of 35 stages with no deferrals, including the real-engine regression, all three packaged native gates, CodeScene, and cleanup. See target/verification/compare-growth-final/summary.json and 30-acceptance-test-rust.log. In a separate packaged native smoke, a real TypeScript fixture rendered the changed symbol and caller, the graph change list, and JSON detail, then returned to the list. Screenshots and trace are under target/verification/compare-growth-native. The owned fixture and process were cleaned up.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The three Compare virtual lists now configure Rune growth on their inner columns. The regression reproduced the original panic before the fix and passed afterward. The full selected run and three-view native smoke passed. All three affected Rust files score 10.0 in CodeScene.
<!-- SECTION:FINAL_SUMMARY:END -->
