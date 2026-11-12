---
id: TASK-52
title: Define shared graph-context workflow and journey acceptance
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T18:51:00Z'
labels: []
dependencies: []
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
type: feature
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents need a portable, bounded graph workflow before host packages ship. Add integrations/shared/graph-context.md as the canonical instructions described in docs/agent-integrations.md. Implement status-first, generation-bound symbol resolution, ambiguity prompts, directed depth-one graph sections (100 nodes/300 edges), 20-hit searches and 8 KiB rendered summaries preserving omissions and provenance. Keep refresh explicit and source/graph semantics CLI-owned. Add reproducible fixture journeys covering unavailable flow, stale/missing/unverifiable graphs, generation replacement, repeated requests and unavailable MCP; document exact expected outcomes. Verify real MCP/compiler fixtures rather than mocked graph logic, and run full verification for executable acceptance tooling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All documented journeys have reproducible expected results against the real CLI-backed MCP, including repeated-context suppression and stale/unverifiable fallback.
- [ ] #2 The canonical workflow never implicitly refreshes, combines generations, chooses an ambiguous symbol silently or claims possible calls were observed execution.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Implement the canonical workflow and budget/deduplication policy in integrations/shared/graph-context.md using docs/agent-integrations.md.
2. Record real compiler-backed MCP acceptance journeys including ambiguity, flow_unavailable, stale/unverifiable states and generation mismatch.
3. Define exact host-visible expected behavior and reusable installation journey inputs for both packages.
4. Run applicable full verification for executable acceptance tooling and review source through CodeScene; publish evidence.
<!-- SECTION:PLAN:END -->
