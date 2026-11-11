---
id: TASK-38
title: Focus search results at their own graph level
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T08:20:00Z'
updated_date: '2026-11-11T08:27:00Z'
labels: []
dependencies: []
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Search activation currently drills classes into their methods and can leave the graph grouping inconsistent with a method result. The user wants the found class or method itself selected and centered at its own level.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Activating a class search result selects and centers a class-level node without opening its methods.
- [x] #2 Activating a method search result selects and centers a method-level node, including when switching from a class search.
- [x] #3 Reveal filters and a single navigation-history entry are preserved, with runtime regression coverage.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Use class-group focus for class results and explicit method grouping for method results while preserving one checkpoint. 2. Assert selected node kind, identity and camera centering through production runtime tests, including level switches and layer history. 3. Update search documentation, run focused and required checks, CodeScene review and packaging.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Regression tests first reproduced four failures in the previous behavior. Changed class activation to Group focus with Project grouping; method reveal explicitly switches to Method grouping after its single checkpoint. Reused existing visual selection and layout-completion centering; preserved filter reveal and existing class-card member previews. Twelve modal tests now pass, checking selected identities, class vs method graph node kinds, camera center within 0.1 points, level transitions and history restoration. Normal desktop suite:179 passed. Check and strict workspace Clippy pass. Both changed Rust files CodeScene10; safeguard passes2 eligible files. Independent review has no actionable findings. Packaged app and ZIP rebuilt; class-result.png and method-result.png visually inspected from real-time packaged Metal replay. Full debug workspace command stops at the three existing release-only benchmark guards (183 other desktop tests pass); all three pass in the separate release run. Initial missing engine prerequisites were resolved by rebuilding it. Evidence: evidence/ui/search-modal/README.md.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Search activation now selects and centers the found symbol at its own graph level: class cards for class hits and Method grouping for method hits. Preserved single-entry history and hidden-symbol reveal. Runtime regressions, visual packaged checks, compiler/Clippy, CodeScene and release benchmarks verified.
<!-- SECTION:FINAL_SUMMARY:END -->
