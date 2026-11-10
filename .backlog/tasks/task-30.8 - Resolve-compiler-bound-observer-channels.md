---
id: TASK-30.8
title: Resolve compiler-bound observer channels
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T14:44:00Z'
labels: []
dependencies:
  - TASK-30.7
parent_task_id: TASK-30
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Registration and actual collection invocation connect only related receivers
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
First add observer corpus source and commit its graph using the evidence-stage engine. Then recognize exact built-in Set collections, direct registrars and actual callback invocations. Support module const, private static readonly and proven closed-singleton private readonly fields; leave ambiguous receivers/mutations unresolved. Share invocation-specific targets between graph and flow; record registration/invocation evidence; verify negative controls and graph-only deltas.
<!-- SECTION:PLAN:END -->
