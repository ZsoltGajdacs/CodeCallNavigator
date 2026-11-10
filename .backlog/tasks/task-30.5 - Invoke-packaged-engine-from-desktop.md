---
id: TASK-30.5
title: Invoke packaged engine from desktop
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T14:15:00Z'
labels: []
dependencies:
  - TASK-30.4
parent_task_id: TASK-30
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Desktop discovery and analysis use the subprocess with recovery and cancellation
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Cut desktop to validated subprocess protocol; preserve linked sources, cancellation and last successful cache; package both distributions and verify real packaged output equivalence.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Desktop uses independently packaged engine subprocess; normal dependency tree excludes engine library. Real TS roundtrip and linked sources, C# linked-source machine protocol, failed refresh, graceful/forced cancellation, terminal-hang timeout and UI discovery pass. Packaged app and standalone produce identical single graph JSON. CodeScene clean; full workspace Clippy passed after four behavior-preserving baseline lint fixes.
<!-- SECTION:FINAL_SUMMARY:END -->
