---
id: TASK-82
title: Expose shared engine continuous analysis sessions
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
labels: []
dependencies:
  - TASK-81
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Initial analysis and automatic refresh currently use separate lifecycle paths. Slice 4; engine owns recovery for all clients.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Public CLI and versioned machine protocol support initial continuous sessions without saved graph
- [ ] #2 External edits coalesce with capped backoff and no retry limit
- [ ] #3 No duplicate lifecycle invocation on unchanged external revision; cancellation reaps children
<!-- AC:END -->
