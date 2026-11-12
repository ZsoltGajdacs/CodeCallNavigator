---
id: TASK-53
title: Package Claude Code graph-context skill
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T18:51:00Z'
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
Depends on the shared workflow task. Build integrations/claude-code with .claude-plugin/plugin.json and skills/graph-context/SKILL.md; copy canonical shared instructions into the installed root at packaging time and detect drift. Bundle no hooks or binaries initially; document separate personal per-project stdio MCP registration, approvals and removal. Validate the manifest then exercise explicit and implicit invocation in an isolated Claude configuration against a real compiler graph. Cover missing/stale/unverifiable graph, repeated requests, disabled MCP and full install/uninstall without deleting artifacts. Record host version and macOS acceptance; do not claim other platforms. Run full packaging verification and CodeScene for executable changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Isolated Claude Code install, explicit/implicit skill journeys and uninstall have recorded evidence with no production configuration changes.
- [ ] #2 The package preserves canonical workflow behavior, has no hooks or graph implementation, and documents project-scoped MCP setup/removal.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consume TASK-52 canonical workflow in the Claude plugin package with drift validation.
2. Add host metadata and local per-project MCP setup/removal guide; no hooks/binaries.
3. Validate manifest and isolated real-host install, explicit/implicit invocation, repeated context, disabled MCP and uninstall.
4. Record exact surface/version limitations and run full packaging checks plus CodeScene for source changes.
<!-- SECTION:PLAN:END -->
