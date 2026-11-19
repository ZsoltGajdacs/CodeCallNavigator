---
id: TASK-94
title: Continue verification when the macOS desktop is locked
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19T05:46:00Z'
updated_date: '2026-11-19T05:48:00Z'
labels: []
dependencies: []
documentation:
  - doc-21
type: enhancement
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A locked console blocks real-window acceptance. Continue runnable verification and preserve incomplete evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Desktop acceptance and native pan preparation are deferred if the desktop is locked or unavailable.
- [x] #2 Deferred gates are recorded and later stages and cleanup continue; genuine failures take precedence.
- [x] #3 Script-level fixture, subprocess and dry-run checks pass and doc-21 describes the outcome.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add a read-only console probe and guard live desktop helpers.
2. Propagate deferred outcomes through the inventory, runner and reports.
3. Validate scripts with fixtures, dry runs and Code Health. No live native verification in this task.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
1. Recorded the approved implementation and script-only validation scope. Backlog creation needed native access to its Git metadata lock.
2. Added console-state guards to the three real-window acceptance helpers and the native pan profiler. Independent review found malformed plist and category-count gaps, which were fixed and rechecked.
3. Verified fixture and subprocess behavior, syntax, inventory, dry-run determinism, and the three desktop gate markers. CodeScene reviewed all 14 changed Python files at 10.0. The existing CodeScene descendant-cleanup script test passed on rerun with native process-table access. One existing selector fixture test still fails because its temporary checkout omits pan_profile.py.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Locked or unavailable desktops defer real-window acceptance and native pan measurements. Remaining verification and cleanup continue. A run with deferrals exits 3 as incomplete unless a genuine failure makes it exit 1. Validation was limited to scripts as requested, with live desktop behavior left to subsequent verification runs. No commit or push.
<!-- SECTION:FINAL_SUMMARY:END -->
