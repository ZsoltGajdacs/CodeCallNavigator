---
id: TASK-91
title: Retain and prepare local graph snapshots across worktrees
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-18T22:01:00Z'
updated_date: '2026-11-19T01:21:00Z'
labels: []
dependencies: []
type: feature
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reuse locally analyzed revisions across linked worktrees without graph commits or infrastructure changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Eligible analyses retain verified bounded bundles in the approved user cache.
- [ ] #2 Explicit preparation reports progress and cleans up a detached worktree on success, failure and cancellation.
- [ ] #3 CLI, MCP and desktop expose equivalent lookup and preparation.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify bounded content-addressed storage and eviction using actual encoded graphs.
2. Bind eligibility to exact Git commit and checked analysis observations at shared publication.
3. Add detached-worktree preparation and common CLI protocol, then expose equivalent CLI, MCP and desktop controls.
4. Run linked-worktree and client acceptance plus required quality gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented versioned user-local bundle retention, exact Git input checks, observed dependency identity, and detached preparation. CodeScene reviewed cache/storage and preparation code. First sandboxed verification could not read the benchmark profile; the outside-sandbox run exposed compile and Clippy issues that were fixed. Full selector rerunning.

TypeScript and Maven probes prepared, retained, reused, and cleaned up real detached graphs. The tracked-symlink regression probe published only a working graph. Verified store eviction and corruption recovery in the previous full run. Hardened generated-source ownership and skipped Git subprocesses for non-repository refreshes. Final selector run is changes-m3_yrcju.

Latest full run changes-m3_yrcju passed compilation, Clippy and unit checks. Rust integration stopped on three desktop automatic-refresh settle timeouts despite removing Git process launches for non-repositories. The latest compiler-backed tracked-link probe passed. The C# preparation probe could not certify owned descendant cleanup on this host. Worktree recovery confirmed inactivity and removed the probe checkout. Keep this task In Progress pending the full gate.
<!-- SECTION:NOTES:END -->
