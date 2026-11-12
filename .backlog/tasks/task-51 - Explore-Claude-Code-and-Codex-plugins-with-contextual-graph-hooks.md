---
id: TASK-51
title: Explore Claude Code and Codex plugins with contextual graph hooks
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T13:48:00Z'
updated_date: '2026-11-12T18:57:00Z'
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
  - docs/mcp.md
  - docs/agent-integrations.md
  - evidence/agent-integrations/README.md
type: spike
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An MCP endpoint alone still requires agents to know when and how to ask for graph context. Explore companion integrations for Claude Code and Codex that make relevant graph sections available while an agent searches, reads and edits a codebase, including hooks where supported.

Evaluate each host independently rather than assuming equivalent plugin or hook capabilities. This draft covers discovery and a recommended integration experience; it does not commit to shipping a plugin or automatic context injection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A capability comparison verifies current Claude Code and Codex support for plugins, MCP configuration and relevant lifecycle hooks against official documentation, including platform/version constraints and fallbacks.
- [x] #2 Representative code exploration and editing journeys show when relevant graph context would be requested or supplied, how the scope is chosen and how context size, duplication and relevance are controlled.
- [x] #3 The report defines the proposed installation/configuration and opt-in experience, project boundaries, freshness signaling and behavior when hooks, MCP or graph analysis are unavailable.
- [x] #4 A recommendation distinguishes shared MCP capabilities from host-specific integration needs and records open decisions, validation scenarios and implementation follow-up tasks.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify official host capabilities and local versions independently.
2. Exercise packaged MCP with a real compiler fixture and record bounded protocol evidence.
3. Compare explicit skills, skill-guided MCP and opt-in hooks; document journeys, installation, freshness and fallback.
4. Publish docs/agent-integrations.md and evidence/agent-integrations; create bounded implementation follow-ups.
5. Run documentation verification and finalize all four criteria with evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Research report and real MCP protocol/isolated host configuration evidence completed. Recommended shared skill plus separate host packages, hooks deferred to optional experiments. Follow-ups TASK-52 through TASK-56 created under approved plan. Full plugin/model invocation and interactive trust acceptance remain explicit follow-up gates; no runtime source changed.

AC1: docs/agent-integrations.md dated capability matrix with official sources, installed versions and explicit unverified surfaces. AC2: workflow/journey table and budget/deduplication policy. AC3: project-bound setup/removal, opt-in, stale/unverifiable and unavailable-service fallbacks. AC4: recommendation and dependency-linked TASK-52..56 with implementation plans. Evidence: protocol.md records 16 real compiler-backed tool operations plus missing-engine failure; hosts.md records seven successful isolated configuration/manifest operations. Documentation-only verify-light passed at target/verification/light-ilt3qith; no runtime source changed.

Research and full companion-integration delivery use branch codex/agent-integrations, based on updated origin/main (0f41553). Keep subsequent feature implementation on this branch.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Archived DRAFT-1 and promoted DRAFT-2 to TASK-51. Completed official-source comparison and implementation handoff, real TypeScript/MCP protocol evidence and isolated host configuration probes. Recommend shared graph-context skill with independent Claude/Codex packages; optional hooks remain experiments. Created TASK-52..56 with plans and dependencies. Full host plugin invocation/trust behavior remains explicitly unverified and gated in follow-ups.
<!-- SECTION:FINAL_SUMMARY:END -->
