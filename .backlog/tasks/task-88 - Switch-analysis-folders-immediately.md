---
id: TASK-88
title: Switch analysis folders immediately
status: Done
assignee:
  - '@codex'
created_date: '2026-11-18T17:07:00Z'
updated_date: '2026-11-18T17:46:00Z'
labels: []
dependencies: []
type: bug
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The desktop keeps the previous project selected and displays its graph after confirming a different folder. Scope is the desktop workspace lifecycle and its existing CLI client.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A confirmed folder switch immediately selects the target and clears the prior graph, then displays the saved graph if present.
- [x] #2 Missing graphs are a normal empty state and a graphless selected folder survives restart.
- [x] #3 Late project work cannot repopulate a superseded workspace and same-folder analysis preserves the last successful graph.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extend real desktop acceptance with the graphless and saved project transitions.
2. Centralize folder commits, graph teardown and saved snapshot opening.
3. Verify startup, supersession and same-project refresh with the repository selector and Code Health.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented desktop-only project selection, shared graph retirement and saved snapshot opening. A confirmed folder switch clears the old graph and navigation before asynchronous discovery and snapshot loads. Existing desktop sessions recover the selected folder and known linked roots.
Verification progression: changes-3w1xblsq found a Clippy issue and changes-gd7h9y3y caught a build-selection restoration fixture relying on the former draft-edit reset. Both were corrected. The final run changes-6wr2tqpo passed compilation, Clippy, all selected UI unit and integration tests (including real CLI/Rune folder switches, A → B → A, same-folder failure/cancellation, and graphless restart), packaging, signature, engine smoke check and cleanup. Reviewed the graphless Rune/Metal capture in target/verification/folder-switch-captures.At4e30/graphless-project.png. CodeScene MCP reports 10.0 for reviewed modified Rust sources. No Sonar config.
Remaining gate: the C# native desktop acceptance could not register the owned AeroSpace window. Read-only console inspection confirmed CGSSessionScreenIsLocked=true. The later two native gates and the selector CodeScene stage did not run. Leave task In Progress pending an unlocked console and a complete verification run.

Unlocked-console rerun changes-bj_pkhvc passed all 57 selected stages, including the C# desktop, saved-flow comparison, and flow expansion native gates, CodeScene, and category cleanup. Timing in seconds: compilation 198.365, setup 16.968, tests 206.592, cleanup 8.102.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Confirming a new analysis folder immediately updates the desktop selection and clears the previous graph. The desktop opens a saved snapshot when available and restores graphless selections at startup. Same-folder reanalysis keeps its previous graph during the run and after a failure or cancellation. All selected verification stages passed in target/verification/changes-bj_pkhvc, including all three packaged native acceptance gates.
<!-- SECTION:FINAL_SUMMARY:END -->
