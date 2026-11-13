---
id: TASK-60
title: Expose typed CLI-owned saved-input freshness observations
status: To Do
assignee: []
created_date: '2026-11-13T08:05:00Z'
updated_date: '2026-11-13T08:22:00Z'
labels: []
dependencies:
  - TASK-59
  - TASK-63
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Saved edits currently wait for desktop polling and freshness is reduced to a boolean. Deliver prompt verified observations while keeping graph loading and watcher ownership separate; saved files only. Extract typed core contracts, an engine observer and explicit capability conversion beside their first consumers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CLI exposes generation-bound match/stale/unverifiable observations and contextual failures without changing existing one-shot query guarantees.
- [ ] #2 Measured supported workloads meet one-second p95 saved-edit stale detection or expose an explicit unsupported/limited mode; commands and at least 20 warm samples are retained.
- [ ] #3 Add/delete/rename/atomic saves, missing inputs, linked roots and dropped-event reconciliation preserve input coverage and output exclusions without refresh loops.
- [ ] #4 Artifact/index reuse remains query-executor owned; filesystem observation and verification reuse core manifests without desktop/MCP watcher implementations.
- [ ] #5 Real compiler and filesystem journeys plus full verification and CodeScene pass, including backward-compatible capability negotiation.
- [ ] #6 C# continuous-freshness support is released only after TASK-63 resolves or explicitly documents the measured cold/warm input-coverage discrepancy; unmapped TypeScript ancestor probes remain unverifiable rather than being dropped.
<!-- AC:END -->
