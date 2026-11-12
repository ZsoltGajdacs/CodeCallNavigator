---
id: TASK-54
title: Package Codex graph-context skill
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T19:27:00Z'
labels: []
dependencies:
  - TASK-52
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
type: feature
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Package the canonical graph-context workflow for Codex so agents invoke ccnavigator directly through terminal PATH. The user explicitly replaced the earlier MCP route: no MCP registration or executable path configuration is required. Deliver a portable root plugin.json, self-contained graph-context skill copied from canonical instructions with drift detection, and local marketplace/install/removal instructions. Ship no hooks, binaries, graph reader, algorithms or background refresh. Validate real isolated CLI installation, explicit/implicit invocation, repeated context and fallbacks. Validate desktop separately where isolation is available; otherwise state the unsupported/unverified surface precisely. Run full packaging verification and CodeScene for source changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Codex CLI/desktop support claims each have isolated installation, invocation and removal evidence or an explicit unsupported-surface limitation.
- [ ] #2 The self-contained package invokes ccnavigator from PATH without MCP setup or binary paths, has no hooks or automatic analysis, and cannot silently drift from the canonical workflow.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consume the shared package builder and canonical workflow in a portable Codex manifest/skill with a local catalog.
2. Document normal local installation, explicit/implicit invocation and removal with ccnavigator on PATH and no MCP configuration.
3. Verify isolated CLI installation/conversations/fallbacks/removal; record the precise desktop isolation limitation unless separately proven.
4. Review, run focused package tests and combined full packaging verification, record evidence and finalize only proven criteria.
<!-- SECTION:PLAN:END -->
