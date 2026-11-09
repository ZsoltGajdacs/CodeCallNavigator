---
id: TASK-11
title: Move graph filters onto the canvas
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T07:00:00Z'
updated_date: '2026-11-09T07:04:00Z'
labels: []
dependencies: []
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Graph filters occupy a sidebar drawer and expanded-group controls consume space; keep filters accessible over the graph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Five working filters appear in one accessible row over the graph at normal and minimum sizes.
- [x] #2 Remove the filter drawer and expanded-groups list while preserving group controls and saved preferences.
- [x] #3 Overlay interactions preserve camera behavior and UI tests and Code Health checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Move filters to a canvas overlay and remove obsolete drawer-only behavior. 2. Verify layout, accessibility, interactions, and saved preferences. 3. Run tests and Code Health checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented sibling overlay and legacy drawer fallback; removed obsolete drawer and collapse action. UI regression suite: 47 passed; focused overlay tests passed after assertion cleanup. Inspected Metal headless minimum-size light and dark captures. CodeScene reviewed changed source files and pre-commit gate passed; icons contain no scoreable methods. Sonar inactive because project configuration is absent. Initial sandboxed Metal captures failed; captures succeeded with device access.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Moved all graph filters into an accessible responsive canvas overlay, retaining filter and group behavior and saved preferences. Verified UI interactions, minimum-size light/dark rendering, session compatibility, formatting, and CodeScene quality gate.
<!-- SECTION:FINAL_SUMMARY:END -->
