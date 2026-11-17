---
id: TASK-83
title: Reuse TypeScript builder programs across session updates
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
labels: []
dependencies:
  - TASK-82
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current TypeScript worker serves one request and loses compiler incremental state. Slice 5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Repeated updates use project compiler builder APIs with noEmit
- [ ] #2 Compiler configuration and reference changes invalidate programs correctly
- [ ] #3 Real supported compiler versions preserve graph semantics and cancellation
<!-- AC:END -->
