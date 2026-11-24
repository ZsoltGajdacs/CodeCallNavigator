---
id: TASK-110
title: Fix Compare class activation and add diagram depth
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24T10:27:00Z'
updated_date: '2026-11-24T12:35:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 119000
---

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Outline hover and focus never open source previews.
- [x] #2 Class activation shows saved diagrams and stale Worktree provenance.
- [x] #3 Shared diagram depth also controls saved source eligibility.
- [ ] #4 Deterministic verification, native acceptance and CodeScene checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fix outline activation and preview behavior.
2. Share bounded class neighborhood, depth selector, and graph layout ordering.
3. Verify stale Worktree, deeper class previews, compatibility and Code Health.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented preview-free outline hover and focus, class activation against the bound saved graphs with Worktree freshness, and the shared 1–64 class diagram depth for desktop, CLI, and MCP.

The stale Worktree regression, development units, and packaged CLI/MCP parity passed. Native Compare passed the row-hover checks, found a canvas-preview focus carryover, and the follow-up unit regression passed after fixing it. Run changes-q5ql2q8e passed 101 stages, including the integration and performance checks. macOS was locked for three native gates. The CodeScene safeguard passed after its findings were resolved.

The user asked to stop the final rerun and verify manually. Interrupted run changes-65n4rks2 passed Cargo check and Clippy and performed cleanup. Manual native acceptance remains pending. The task stays In Progress.

The user confirmed the Compare changes work in manual verification and requested closure and a local commit. This accepts closure with the full automated final run incomplete. Acceptance criterion 4 remains unchecked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compare keeps the change outline free of hover and focus source previews, opens bound saved class diagrams with stale Worktree provenance, and uses a single 1–64 depth control for both panes and their saved source previews. The user accepted the behavior after manual verification. Cargo check, Clippy, the stale Worktree integration regression, packaged CLI/MCP parity, and the CodeScene safeguard passed. The user stopped the final automated rerun and accepted closure with that run incomplete.
<!-- SECTION:FINAL_SUMMARY:END -->
