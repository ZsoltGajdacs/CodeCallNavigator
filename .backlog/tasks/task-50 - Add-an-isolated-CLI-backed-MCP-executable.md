---
id: TASK-50
title: Add an isolated CLI-backed MCP executable
status: To Do
assignee: []
created_date: '2026-11-12T13:17:00Z'
updated_date: '2026-11-12T15:05:00Z'
labels: []
dependencies:
  - TASK-49
references:
  - DRAFT-1
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expose the desktop-proven public CLI interface to agents without linking MCP to engine, core, desktop, analyzer or storage implementation. Independently installable MCP communicates only with the CLI executable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Separate Rust MCP package exposes graph query tools and explicit refresh over stdio using the official SDK.
- [ ] #2 MCP imports no core, engine or desktop libraries including tests and performs no direct graph/source/database access or graph algorithms.
- [ ] #3 Configured executable/project/roots, compatibility checks, structured metadata/errors and cancellation work through CLI subprocesses.
- [ ] #4 Real-CLI end-to-end coverage, dependency-boundary checks, packaged installation, full verification and CodeScene pass.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User explicitly instructed during TASK-49 implementation: finish and commit desktop work, then stop. MCP implementation must remain unstarted until a later request. Task remains To Do; no MCP package or dependencies have been created.
<!-- SECTION:NOTES:END -->
