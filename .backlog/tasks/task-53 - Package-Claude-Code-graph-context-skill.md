---
id: TASK-53
title: Package Claude Code graph-context skill
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
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Package the canonical graph-context workflow for Claude Code so agents can invoke ccnavigator directly through terminal PATH. The user explicitly replaced the earlier MCP route: no MCP server registration or executable path configuration is required. Deliver the host manifest, a self-contained skill copied from the canonical body with drift detection, local marketplace/install/removal instructions, and real isolated host evidence. Ship no hooks, binaries, graph reader, algorithms or background refresh. Cover explicit and implicit selection, ambiguity, missing/stale/unverifiable graphs, repeated scope, unavailable terminal command, and removal preserving source/artifacts. Record exact supported host version and macOS limits; full packaging verification and CodeScene apply to source changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Isolated Claude Code install, explicit/implicit skill journeys and uninstall have recorded evidence with no production configuration changes.
- [ ] #2 The package preserves the canonical direct ccnavigator workflow, has no MCP setup, hooks or graph implementation, detects drift and documents installation/removal.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Package the canonical workflow in a minimal Claude manifest/skill with a local catalog.
2. Add packaging-time canonical-body/drift validation and self-contained archive checks; write host installation/removal instructions without binary paths or MCP setup.
3. Exercise the actual package in isolated CLI configuration and authenticated real conversations, including implicit selection and unavailable-command fallback.
4. Review, run focused package tests and required full verification, record evidence, and finalize only proven criteria.
<!-- SECTION:PLAN:END -->
