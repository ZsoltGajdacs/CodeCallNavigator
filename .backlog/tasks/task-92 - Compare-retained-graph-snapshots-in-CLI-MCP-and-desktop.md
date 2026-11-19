---
id: TASK-92
title: Compare retained graph snapshots in CLI, MCP, and desktop
status: Done
assignee:
  - '@codex'
created_date: '2026-11-18T22:02:00Z'
updated_date: '2026-11-19T08:48:00Z'
labels: []
dependencies:
  - TASK-91
type: feature
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use retained exact revisions directly in existing graph and function flow comparisons.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Read-only cache and committed endpoints have stable provenance and binding.
- [x] #2 CLI, MCP and desktop expose equivalent variant selection and graph and flow results.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Separate ref resolution and shared bundle inspection.
2. Add cache-backed graph and flow endpoints and bind provenance.
3. Wire equivalent CLI, MCP and desktop operations and verify real linked-worktree journeys.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented cache-aware endpoint resolution and CLI/MCP/desktop graph and flow comparison, with explicit variant selection, progress and cancellation. Refactored bundle resolution and MCP process orchestration. Added real engine, MCP stdio and desktop-controller journeys. Full selector rerunning.

The previous full run passed new CLI and desktop acceptance. MCP flow acceptance picked the module instead of the fixture function; corrected the selection and confirmed both sides are ready through real MCP stdio. Final selector run is changes-m3_yrcju.

Final selector stopped before acceptance and performance because of existing desktop automatic-refresh journey timeouts. Earlier CLI and desktop snapshot journeys and the corrected real MCP flow probe provide direct feature evidence. Keep this task In Progress pending full verification.

Full final verification changes-7bkmhbjx passed the packaged CLI, linked-worktree, desktop-controller and MCP stdio comparison acceptance tests and performance. The real-window flow comparison gate and two other real-window gates were not performed because the Mac console was unavailable. CodeScene stopped on storage.rs because it returned no numeric score. Overall result failed, so leave this task In Progress.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Retained and committed graph endpoints preserve revision provenance and comparison binding. CLI, MCP stdio and desktop-controller acceptance passed for variant selection, graph differences and function flows in changes-7bkmhbjx. Performance passed. The user waived the three deferred real-window gates and directed task closure. The full selector still records an unscored CodeScene review for storage.rs, whose only change was a module declaration. The pre-commit Code Health safeguard passed for the closeout changes.
<!-- SECTION:FINAL_SUMMARY:END -->
