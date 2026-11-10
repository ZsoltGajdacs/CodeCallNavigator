---
id: TASK-30.4
title: Publish one canonical project graph artifact
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T14:12:00Z'
labels: []
dependencies:
  - TASK-30.3
parent_task_id: TASK-30
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Portable deterministic .codegraph/graph.json is atomically replaced under a project lock
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add canonical artifact and atomic whole-analysis lock; adapter semantic export keys; exact output exclusions; versioned CLI/protocol; verify relocation, identity stability, old-file recovery and config tracking.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Canonical project artifact, stable export identities, atomic whole-analysis locking and precise output exclusions verified by core tests and real Maven/Gradle/C#/TS/mixed repeat+relocation comparisons. Gradle display-root normalization fixed after real comparison. Git comparator and machine protocol have actual-executable tests; CodeScene safeguard passes.
<!-- SECTION:FINAL_SUMMARY:END -->
