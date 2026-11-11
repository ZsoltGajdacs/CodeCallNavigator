---
id: TASK-41
title: Refresh vendor sources and fix Rune commit meter warning
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-11T11:04:00Z'
updated_date: '2026-11-11T11:11:00Z'
labels: []
dependencies: []
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update Git vendor source tips while preserving integration patches; release builds report unused CommitMeter methods.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All Git vendor revisions match verified upstream default branch tips
- [x] #2 Local upstream changes remain reproducible via checksum-pinned patches
- [x] #3 Release warning is fixed and integration checks recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify vendor diffs and fetch default branch tips. 2. Rebase Rune integration patch onto latest dev; preserve applicable migration checkpoints and compile debug-only meter methods only in debug. 3. Verify patch installation/migration, release and debug builds, workspace tests, bootstrap and packaging. 4. Independent review, CodeScene checks, and record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner and explorer reviews completed. Verified Rune and CallAtlas diffs exactly equal pinned patches. Upstream default tips: Rune bba30a61cfcec6683e3fdedf156a92dae89d8b04; CallAtlas and java-callgraph2 unchanged.

Rebased Rune to bba30a61; preserved visibility hooks and moved observer wiring. Added debug-only meter cfg and narrowed three module-local helpers to fix latest upstream visibility warnings. Refreshed patch/checkpoints/checksums and Cargo.lock. Independent reviewer/tester passed patch correctness, all 7 dependency tests, exact checkout/checksum verification, and 4 observer tests. Warning-free final cargo check, strict Clippy, bootstrap, release packaging, normal root tests and 76 Python tests passed. Release dense-canvas benchmark passed. CodeScene reviewed six edited/rebased source files; findings are pre-existing upstream complexity outside changed methods. Root safeguard passed with zero eligible source files (changes are patch artifacts). Sonar unconfigured. Full include-ignored debug run failed on 3 release-only benchmark prerequisites; continuation excludes those and is running compiler acceptance.

Release package and dense benchmark passed without warnings. Normal root suite: 329 passed, 69 ignored; Python suite 76 passed. Full acceptance continuation stopped after Maven/Gradle checks at java_duplicates recurring_billing_retains_duplicate_generated_declarations: CODECALLNAVIGATOR_RECURRING_BILLING is unset. Later binaries not reached. Evidence: evidence/verification/vendor-refresh-2026-11-11.md. Implementation complete; task remains In Progress because full validation has unmet prerequisites and no waiver was assumed.
<!-- SECTION:NOTES:END -->
