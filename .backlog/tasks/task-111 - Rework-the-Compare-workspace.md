---
id: TASK-111
title: Rework the Compare workspace
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-24T13:05:00Z'
updated_date: '2026-11-24T14:51:00Z'
labels: []
dependencies: []
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement selected design picture 2 for desktop Compare, with pane-local revision and snapshot controls and the shared Worktree analysis lifecycle.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Changes sidebar and paired revision headers match the selected design.
- [ ] #2 Commit and Worktree snapshot lookup and preparation work from Compare, and the normal graph shares the published Worktree generation.
- [ ] #3 Canvas controls are local to each pane, with shared class depth and Git-first source browsing.
- [ ] #4 Retired desktop modes are removed and deterministic, native and Code Health verification is recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Bind pane snapshot actions to the existing comparison and analysis lifecycles. Compose the selected workspace and share camera control rendering. Remove superseded desktop modes and exercise the real Git and native journeys.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented design picture 2, including the file-first Changes sidebar, pane-local snapshot headers and camera overlays, shared class depth, and automatic method pairing. Worktree preparation uses normal build review and graph publication. Removed superseded desktop modes and updated the desktop and native verification guides.

Reviewer and tester exercised revision binding, retained source and camera navigation. The final selector run changes-0rn14gyc passed 98 stages including Cargo check, Clippy, unit, integration, compiler-backed snapshot acceptance, packaging and installed CLI/MCP journeys. Three native gates (C# desktop, Compare desktop and flow expansion desktop) were deferred because macOS was locked. The private Maven test passed using an isolated copy of committed revision fe211954b. Recorded stage timings were 315.2 s compilation, 16.0 s setup, 1137.8 s tests, and 56.6 s cleanup.

The final CodeScene stage rejected an unscored declaration-only module with a test-only re-export. Removed the redundant re-export. The same CodeScene stage then passed on 57 files, with evidence in target/verification/changes-0rn14gyc/codescene-declaration-recheck.json. The seven development stages passed again in changes-l8mofzbp. Final acceptance remains pending because the Mac console is still locked.
<!-- SECTION:NOTES:END -->
