---
id: TASK-54
title: Package Codex graph-context skill
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
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Depends on the shared workflow task. Build integrations/codex with portable root plugin.json and skills/graph-context/SKILL.md; package canonical shared instructions without external symlinks and check drift. Ship no hooks or binaries initially; use separately configured project-bound stdio MCP. Document trusted project config plus user-scope alternative. Validate local marketplace installation, explicit skill mention and description-based selection separately on CLI and desktop where available; unsupported surfaces remain unverified. Exercise real compiler graph journeys, disabled MCP, repeated context, install/removal and normal trust. Record exact versions. Full packaging verification and CodeScene apply to executable changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Codex CLI/desktop support claims each have isolated installation, invocation and removal evidence or an explicit unsupported-surface limitation.
- [ ] #2 No hooks, automatic refresh, graph readers or production configuration mutations are included, and packaged shared instructions cannot drift silently.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consume TASK-52 canonical workflow in a portable Codex package with drift validation.
2. Add trusted-project and user-scope stdio configuration/removal instructions; no hooks/binaries.
3. Verify isolated local marketplace installation and real skill journeys separately in CLI and desktop.
4. Verify repeated-context/fallback/uninstall behavior and run full packaging checks plus CodeScene for source changes.
<!-- SECTION:PLAN:END -->
