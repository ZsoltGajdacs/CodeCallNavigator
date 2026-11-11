---
id: DRAFT-2
title: Explore Claude Code and Codex plugins with contextual graph hooks
status: Draft
assignee: []
created_date: '2026-11-11T13:48:00Z'
updated_date: '2026-11-11T13:48:00Z'
labels:
  - exploration
  - plugins
  - agents
dependencies: []
references:
  - DRAFT-1
  - DRAFT-3
documentation:
  - docs/standalone-engine.md
type: spike
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An MCP endpoint alone still requires agents to know when and how to ask for graph context. Explore companion integrations for Claude Code and Codex that make relevant graph sections available while an agent searches, reads and edits a codebase, including hooks where supported.

Evaluate each host independently rather than assuming equivalent plugin or hook capabilities. This draft covers discovery and a recommended integration experience; it does not commit to shipping a plugin or automatic context injection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A capability comparison verifies current Claude Code and Codex support for plugins, MCP configuration and relevant lifecycle hooks against official documentation, including platform/version constraints and fallbacks.
- [ ] #2 Representative code exploration and editing journeys show when relevant graph context would be requested or supplied, how the scope is chosen and how context size, duplication and relevance are controlled.
- [ ] #3 The report defines the proposed installation/configuration and opt-in experience, project boundaries, freshness signaling and behavior when hooks, MCP or graph analysis are unavailable.
- [ ] #4 A recommendation distinguishes shared MCP capabilities from host-specific integration needs and records open decisions, validation scenarios and implementation follow-up tasks.
<!-- AC:END -->
