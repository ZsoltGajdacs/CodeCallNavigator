---
id: TASK-78.2
title: Resolve false stale reports from incidental build artifacts
status: To Do
assignee: []
created_date: '2026-11-20T11:05:00Z'
labels: []
dependencies:
  - TASK-78.1
parent_task_id: TASK-78
type: bug
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Saved read-only freshness still observes broad directory watches. A build can add an unrelated file or directory and trigger stale even when evaluated source membership and consumed graph inputs are identical. Investigate bounded adapter-aware reconciliation or an explicit uncertainty result. Preserve the distinction between scheduling a refresh and reporting a changed graph input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An unrelated artifact does not make the saved graph stale when compiler membership and consumed inputs are demonstrably unchanged.
- [ ] #2 Matching source additions and removals, module resolution changes, and consumed file content changes remain detectable.
- [ ] #3 Report bounded uncertainty when saved evidence alone cannot establish membership.
<!-- AC:END -->
