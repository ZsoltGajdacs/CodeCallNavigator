---
id: TASK-30.6
title: Compare Git graph versions and verify foundation
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T14:21:00Z'
labels: []
dependencies:
  - TASK-30.5
parent_task_id: TASK-30
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Git comparator reports full semantic changes and foundation matches merged master
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Verify all compiler acceptance except documented private/GPU prerequisites; record deterministic repeat/relocation evidence; commit one graph per corpus project; exercise Git comparisons and checkpoint before inference.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Foundation gate passed: all five workloads match frozen master in three additional runs each; repeat/relocation exports match, initial corpus graphs generated for Git history, actual Git comparator tests pass. Full workspace normal and real-toolchain acceptance passed excluding documented private/GPU prerequisites; strict workspace Clippy and CodeScene clean. Packaged outputs equivalent. Evidence in evidence/engine/README.md and JSONL reports.
<!-- SECTION:FINAL_SUMMARY:END -->
