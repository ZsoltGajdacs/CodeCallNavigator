---
id: TASK-101
title: Show actionable snapshot build failures
status: Done
assignee:
  - '@codex'
created_date: '2026-11-20T17:09:00Z'
updated_date: '2026-11-20T18:16:00Z'
labels: []
dependencies: []
type: bug
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compare exposed only JVM warnings when a Maven reactor failed. Maven printed the actionable failures to stdout.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Maven snapshot failures report actionable diagnostics and the retained raw log directory.
- [x] #2 Compare displays the typed snapshot failure and copies diagnostic details and log location.
- [ ] #3 Real Maven and UI regressions pass with the computed verification plan and cleanup.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve native exit status and captured paths and format Maven-owned excerpts from both logs.
2. Carry the existing typed failure through Compare and surface selectable, copyable details.
3. Verify CLI and desktop protocol with real Maven, exercise Compare details, and run required quality and cleanup checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reproduced the reported failure from saved Maven logs. Both reactor failures were in stdout and the JVM warning was in stderr. Independent review confirmed typed native failures, process cleanup, and snapshot state transitions are preserved. Corrected the clipboard test type after the first development compile and verified its category cleanup. Refactored the formatter to consume typed log tails and rechecked all touched Rust files at Code Health 10.0. The Code Health safeguard passed 12 files.

Final selector changes-zcbf35ns reported 55 passing stages and no failures. The real Maven CLI and protocol log-retention regression and both Compare regressions passed. Native C# desktop, Compare, and flow expansion gates were deferred because the macOS console was locked. The run is incomplete pending an unlocked console. All category cleanup passed, and the pinned Recurring Billing fixture clone was removed after worktree and process checks.

The user explicitly requested closure after being told that the three native desktop gates were deferred. Closed with that verification exception recorded. Acceptance criterion 3 remains unchecked because those gates did not run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Maven snapshot failures now expose actionable compiler and reactor diagnostics plus retained build logs. Compare shows and copies typed failure details. Real Maven CLI/protocol and Compare regressions, Code Health, and cleanup passed. Verification changes-zcbf35ns passed 55 of 58 stages. Native C# navigation, Compare, and flow expansion were deferred because macOS was locked.
<!-- SECTION:FINAL_SUMMARY:END -->
