---
id: TASK-78
title: Ignore generated dist directories and identify changed analysis inputs
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-17T11:36:00Z'
updated_date: '2026-11-17T11:56:00Z'
labels: []
dependencies: []
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Fresh secondary Recurring checkout builds successfully but fails input stability checks after Maven creates frontend dist output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Broad discovery and freshness ignore generated dist directories while explicit required inputs remain observed
- [ ] #2 Input-change failures report the exact recorded path and change type
- [ ] #3 Regression coverage proves first-build directory creation does not reject stable sources and authored changes remain rejected
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Extend existing output policy for dist, preserve explicit roots, add typed changed-input context, extend filesystem integration coverage, run deterministic verification and Code Health.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared dist exclusion with explicit required-root preservation; typed InputsChanged now reports recorded path and change kind. Extended existing real-filesystem integration cases (no new test names). Planner/explorer, implementer, reviewer and tester roles completed. Reviewer found directory-before-file deletion assertion; corrected with explicit file snapshot. Both source Code Health reviews 10.0 and final safeguard passed 2 files. Earlier TypeScript work committed separately as 1aa3350 at user request; no full-verification completion claimed for that commit. Selected verification run changes-t8g_tu5g ongoing. No red-before run: deterministic selector requires full computed plan; defect sensitivity reviewed against old exclusion and diagnostic behavior.

Verification: changes-t8g_tu5g passed static/compiler/Clippy/unit checks then lacked dependency env; changes-nepwpxdl lacked Next. Installed pinned fixture dependencies under ignored target/verification/typescript-dependencies and launched changes-re5dhxk8 with private checkout and modules env. Static checks, compiler, Clippy, Rust/Node unit stages and integration prerequisites passed; integration progressed through real Recurring Maven compilation and Java context reading. Active target/verification directory then disappeared externally; runner exited 2 with Verification unavailable (results.json missing). This agent did not delete the directory. Full integration/acceptance completion and new filesystem regression execution cannot be claimed; task stays In Progress. Fix remains uncommitted; earlier TS work committed 1aa3350.
<!-- SECTION:NOTES:END -->
