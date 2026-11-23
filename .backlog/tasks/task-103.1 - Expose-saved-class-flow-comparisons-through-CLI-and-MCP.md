---
id: TASK-103.1
title: Expose saved class flow comparisons through CLI and MCP
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T13:47:00Z'
updated_date: '2026-11-23T17:08:00Z'
labels: []
dependencies: []
parent_task_id: TASK-103
type: feature
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expose class-to-class calls from the exact snapshots that produced a class change row. Direct clients can build the same neighborhood as desktop Compare.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A class change ID opens the same snapshot-bound one-hop directional class graph through CLI and MCP.
- [x] #2 Call counts, certainty, presence, changed links, and stale bindings are covered by verification.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reuse canonical outline ownership in core; add class graph query and capability; forward and validate it in MCP; verify parity.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented a dedicated versioned query and MCP tool with the shared comparison binding. Extended core, retained CLI, and real MCP coverage.

Retained CLI and real MCP parity and stale-binding tests passed in target/verification/changes-9u51k8k0/89-acceptance-test-rust.log. Final verification stopped later in the automatic-refresh broken-pipe case.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the dedicated snapshot-bound class-flow query and MCP tool. Core, saved CLI, real MCP parity, and stale-binding cases passed. Final evidence: target/verification/changes-mdea3q11/summary.json.
<!-- SECTION:FINAL_SUMMARY:END -->
