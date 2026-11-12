---
id: DRAFT-1
title: Explore an MCP interface for CLI-backed graph and execution-flow queries
status: Draft
assignee: []
created_date: '2026-11-11T13:48:00Z'
updated_date: '2026-11-11T13:48:00Z'
labels:
  - exploration
  - mcp
  - agents
dependencies: []
references:
  - TASK-39
  - DRAFT-2
  - DRAFT-3
documentation:
  - docs/standalone-engine.md
  - docs/graph-contract.md
  - docs/function-flowcharts.md
type: spike
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents exploring a codebase need focused structural context without ingesting the entire graph or reconstructing relationships from repeated source searches. Explore an MCP interface backed by the standalone CLI/engine so an agent can ask, for example, "give me the execution flow for Y" and receive the relevant graph section.

This is a discovery draft: establish feasibility, user-facing query semantics and a recommended scope before committing to implementation. Preserve the distinction between source-derived possible execution and observed runtime execution.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An exploration report defines representative agent queries including execution flow for a named symbol and explains symbol ambiguity, caller/callee direction, traversal limits and unsupported flow cases.
- [ ] #2 The report provides example bounded graph-section responses with stable identities, source locations, relationship certainty/provenance, graph generation/freshness and truncation information.
- [ ] #3 Feasibility and tradeoffs of a CLI-backed MCP interface are documented, including reuse of canonical core/engine semantics, transport/lifecycle, project selection, cancellation and error behavior.
- [ ] #4 The report recommends an initial scope and validation scenarios using real project graphs, identifies open decisions and proposes implementation follow-up tasks.
<!-- AC:END -->
