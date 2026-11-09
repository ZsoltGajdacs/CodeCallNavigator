---
id: TASK-10
title: Improve graph selection and navigation
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T06:31:00Z'
updated_date: '2026-11-09T06:37:00Z'
labels: []
dependencies: []
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Immediate expansion and scroll panning make it harder to inspect graph relationships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single click centers an element and emphasizes its direct visible relationships.
- [x] #2 Double click drills into groups or focuses methods without duplicate history.
- [x] #3 Vertical scrolling zooms at the pointer; drag and pinch remain functional.
- [x] #4 Visual selection clears when its element disappears and old sessions remain compatible.
- [x] #5 Guidance and behavior tests are updated.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add transient projection selection, center the selected element, and emphasize direct relationships.
2. Move drill-down to double-click and change vertical scrolling to pointer-centered zoom, preserving reliable double-click hit testing.
3. Update guidance and verify behavior, history, themes, tests, and Code Health.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reviewed current navigation and Rune input handling. Immediate centering can move the clicked card before the second click; implementation must preserve double-click hit testing.

Implemented and ran 42 binary tests, including real-coordinate double-click. Independent review found that non-canvas selections must synchronize visible emphasis; addressing that and CodeScene findings before final verification.

Independent reviewer confirmed selection synchronization after fixes. cargo test --all-targets --offline passed all non-ignored tests; final cargo test --bin codegraph --offline passed 44 tests; cargo clippy --all-targets --offline -- -D warnings passed. CodeScene reviews report no findings in changed source files. Headless Metal fixture renders in light and dark themes verified emphasis. Physical trackpad testing was not available. cargo fmt --all also modified the ignored nested Rune checkout; its pre-existing work was not overwritten.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Graph clicks now select and center elements with direct relationships emphasized, while double-click drills down. Vertical scrolling zooms at the pointer. Selection stays synchronized with navigation and session restoration. Tests, Clippy, CodeScene reviews, and headless light/dark visual checks passed; physical trackpad interaction remains unverified.
<!-- SECTION:FINAL_SUMMARY:END -->
