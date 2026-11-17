---
id: TASK-81
title: Observe generated Java inputs at compilation boundaries
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
labels: []
dependencies:
  - TASK-80
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pre-build snapshots reject legitimate generation and post-build baselines can hide concurrent authored edits. Slice 3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Maven and Gradle supported compile operations capture consumed input evidence
- [ ] #2 Pre-compilation generation succeeds without self-triggering another build
- [ ] #3 Concurrent edits and unsupported compiler behavior retain actionable consistency diagnostics
<!-- AC:END -->
