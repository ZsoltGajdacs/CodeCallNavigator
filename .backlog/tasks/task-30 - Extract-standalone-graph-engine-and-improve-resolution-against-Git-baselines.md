---
id: TASK-30
title: Extract standalone graph engine and improve resolution against Git baselines
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T13:41:00Z'
labels: []
dependencies: []
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved roadmap: preserve merged baseline, extract independent engine, publish one project-local canonical graph and compare Git versions, then precise inference stages after equivalence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Foundation verified against master fb54f5f before resolution changes
- [ ] #2 One deterministic .codegraph/graph.json per project and Git comparator
- [ ] #3 Desktop invokes packaged standalone engine
- [ ] #4 Resolution stages verified in dependency order
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Complete foundation 0A-0F in separate commits, verifying frozen native payload equivalence before inferred relationships. Then implement evidence, observer channels, callable propagation, framework dispatch, HTTP/navigation and incremental inference.
<!-- SECTION:PLAN:END -->
