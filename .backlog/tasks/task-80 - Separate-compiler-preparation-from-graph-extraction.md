---
id: TASK-80
title: Separate compiler preparation from graph extraction
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
labels: []
dependencies:
  - TASK-79
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whole-attempt orchestration unnecessarily couples extraction retries to build lifecycle execution. Slice 2.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Validated compiler results can be reused for extraction without rebuilding
- [ ] #2 Independent unchanged selected builds are reused using existing fingerprints
- [ ] #3 Native build caches and atomic publication are preserved
<!-- AC:END -->
