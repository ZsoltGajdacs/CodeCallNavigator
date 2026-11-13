---
id: TASK-61
title: Adopt typed freshness and new generations in desktop and MCP
status: To Do
assignee: []
created_date: '2026-11-13T08:05:00Z'
labels: []
dependencies:
  - TASK-60
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers currently lose unverifiable/failure context through a boolean, and desktop graph installation resets navigation and auto-fits. Consume the CLI observation contract and split background generation adoption from opening a workspace; MCP remains an independent CLI wire adapter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Desktop distinguishes matching, stale, unverifiable and failed observations without blocking the UI or duplicating engine verification.
- [ ] #2 External artifact publication causes explicit generation reload; valid selected symbols, navigation history and camera survive while deleted selections and obsolete asynchronous jobs are discarded.
- [ ] #3 Opening another project retains explicit workspace reset behavior; linked-root and project bindings cannot leak between generations.
- [ ] #4 MCP forwards negotiated generation-bound observations and errors without graph algorithms, graph caches or watcher ownership; legacy CLI compatibility failures are explicit.
- [ ] #5 Real desktop visual journeys, MCP subprocess acceptance, full verification and CodeScene cover adoption and generation mismatches.
<!-- AC:END -->
