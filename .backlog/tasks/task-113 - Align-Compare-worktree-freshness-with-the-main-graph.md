---
id: TASK-113
title: Align Compare worktree freshness with the main graph
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24T20:18:00Z'
updated_date: '2026-11-24T21:02:00Z'
labels: []
dependencies: []
type: bug
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compare warned about an unmapped-only freshness report that the main graph suppresses for the same worktree generation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Compare suppresses the unmapped-only warning using the shared display rule for the same worktree generation.
- [x] #2 Compare retains warnings when the freshness issue is actionable or matching coverage evidence is missing.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Share the suppression rule and corroborate Compare evidence with the same worktree observation. Run development tests only. User owns manual verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Confirmed the same saved Recurring-billing generation reports unmapped_input in both views. Compare now applies the main graph display status only to matching project, roots, and generation, with corroborating report evidence for suppression. Fresh stale observations override a cached ready status. Development selector passed 228 tests, including the Compare presentation regression (target/verification/changes-z09ej4d4/results.json). CodeScene reviewed all changed Rust files without findings. The user requested development tests only and will verify the native workflow manually. Acceptance remains open for that verification.

User manually verified that Compare shows Snapshot ready after refresh and still warns when the worktree is stale.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compare shares the main graph worktree freshness display rules for the same project, root mappings, and graph generation. It preserves stale and actionable freshness warnings. The development selector passed 228 tests, and the user manually verified both ready and stale states. Verification followed the requested development-only test scope.
<!-- SECTION:FINAL_SUMMARY:END -->
