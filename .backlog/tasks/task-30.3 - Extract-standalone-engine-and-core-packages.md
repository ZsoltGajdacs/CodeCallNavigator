---
id: TASK-30.3
title: Extract standalone engine and core packages
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T13:52:00Z'
labels: []
dependencies:
  - TASK-30.2
parent_task_id: TASK-30
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Independent engine preserves native analysis and has no desktop dependencies
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Relocate unchanged shared contracts/services and analyzers; preserve dependency and resource boundaries; compare complete native payloads against frozen master.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Three-package extraction builds independently, has no desktop dependencies in engine tree and matches frozen master on Maven, Gradle, C#, TS and mixed workloads. Independent review found no binding algorithm changes. Resource and fixture paths adjusted. CodeScene safeguard passes.
<!-- SECTION:FINAL_SUMMARY:END -->
