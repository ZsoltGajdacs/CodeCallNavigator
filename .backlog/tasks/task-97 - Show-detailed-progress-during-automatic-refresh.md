---
id: TASK-97
title: Show detailed progress during automatic refresh
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19T09:20:00Z'
updated_date: '2026-11-19T10:05:00Z'
labels: []
dependencies: []
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A stale graph reopened at a later commit showed only generic analysis activity. Route shared engine progress to desktop, CLI, and explicit MCP analysis.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Desktop shows an engine stage and its elapsed viewing time during automatic analysis.
- [ ] #2 Continuous CLI activity and token-enabled MCP refresh carry the same stages as one-off CLI analysis.
- [x] #3 Progress preserves cancellation and bounded transport.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Share engine stages, forward them through CLI and desktop, reuse MCP progress forwarding, extend real journeys, and run deterministic verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared engine stages, bounded continuous activity delivery, desktop stage timing, and MCP token forwarding. Added real CLI, Rune and MCP regression coverage and a bounded-output unit test. Development selector passed after the Clippy fix. Full verification is running again with discovered fixture paths. A read-only sample of the existing desktop observer showed it spending time in saved-input verification.

Both full attempts passed the stale saved-graph and graphless desktop progress journeys, stage timing and bounded-output unit tests, and Clippy. The second run completed cleanup successfully after the existing app was closed. The existing mixed Java/TypeScript recovery journey timed out twice. The cause was not established, and the selector did not reach CLI/MCP or native acceptance. CodeScene staged safeguard passed with 25 of 25 eligible files checked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Shared engine stage messages now reach automatic desktop activity, continuous CLI activity, and token-enabled MCP refresh. Desktop shows elapsed time for the current stage. Closed at the user’s request with CLI/MCP runtime parity unchecked and remaining composite verification deferred. Evidence: target/verification/changes-2rm3os4s.
<!-- SECTION:FINAL_SUMMARY:END -->
