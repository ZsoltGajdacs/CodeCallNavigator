---
id: TASK-84
title: Integrate thin clients with continuous engine sessions
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-17T15:00:00Z'
labels: []
dependencies:
  - TASK-83
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
MCP and desktop must consume engine recovery rather than implement separate policy. Slice 6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MCP CLI and desktop expose equivalent engine outcomes and codes
- [ ] #2 Desktop opt-in works before first publication and cancellation stops sessions
- [ ] #3 All clients use the new engine contract directly; stale versions fail explicitly and packaging/native acceptance pass
<!-- AC:END -->
