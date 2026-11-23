---
id: TASK-105
title: Show current branch history in Compare commit pickers
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T12:47:00Z'
updated_date: '2026-11-23T12:48:00Z'
labels: []
dependencies: []
references:
  - TASK-104
type: bug
ordinal: 111000
---

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Base and After list commits reachable from the checkout HEAD in newest-first Git date order, with Worktree above After commits.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Capture a single HEAD for both paged revision lists and update the Git history fixture and verification inventory.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compare now lists commits reachable from the checkout HEAD in Git date order and keeps Worktree above the After history. Pagination uses the HEAD captured when Compare opens. All six development stages passed (target/verification/changes-d7r5kpeo/summary.json), and the user confirmed the behavior manually. The user requested development checks and manual verification only, so the updated Git integration test and automated native acceptance were deferred.
<!-- SECTION:FINAL_SUMMARY:END -->
