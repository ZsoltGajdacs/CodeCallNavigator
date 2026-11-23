---
id: TASK-103.2
title: Open class graphs from the desktop change list
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T13:48:00Z'
updated_date: '2026-11-23T17:08:00Z'
labels: []
dependencies:
  - TASK-103.1
parent_task_id: TASK-103
type: feature
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Class rows currently open a source diff while the selected function flow stays visible. Use the paired canvases for class graph changes and keep method rows focused on function flow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Class and method activation display the respective paired canvases and outline hover or focus leaves source closed.
- [x] #2 Native Compare verifies redirected calls, paired layout, and switching back to a method.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Route class and function detail through a typed selection lifecycle; add paired class canvas and native fixture replay.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented a shared selection lifecycle with cancellable Graphviz and a native fixture that redirects calls between classes. Added coverage for obsolete function completions after switching to a class.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Class rows open paired class call graphs and method rows open function flowcharts. Native Compare passed in dark and light themes after placing labels inside the canvas layer. Final evidence: target/verification/changes-mdea3q11/summary.json.
<!-- SECTION:FINAL_SUMMARY:END -->
