---
id: TASK-40
title: Rename CodeGraph to CodeCallNavigator
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-11T09:27:00Z'
updated_date: '2026-11-11T09:36:00Z'
labels: []
dependencies: []
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Adopt approved new identity with clean storage cutover; preserve historical records and external repository identity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Current owned identifiers and packaging use new name
- [x] #2 Fresh storage works without modifying old data
- [ ] #3 Validation and Code Health results recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Rename owned identifiers and paths; review exclusions; validate Rust, analyzers and packaging; record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Applied owned identifier/path rename. Preserved old cache ignore rules after review. Rebuilt Java and Maven adapters successfully. Ignored local benchmark required the same import rename. Rust checks, CodeScene and packaging in progress.

Validation: cargo check and strict workspace Clippy pass; desktop all-target tests pass after building renamed engine; 104 TypeScript tests pass. CodeScene reviewed 277 affected source files: no findings (272 score 10; five declaration-only files unscored). App/ZIP, standalone engine and DMG packaging pass; bundle metadata and executable names verified. Fresh packaged-engine TypeScript analysis publishes new-path graph and clear-cache succeeds. Contract verification valid; external symlinked TypeScript inputs remain unmapped. Full debug include-ignored has release-only performance prerequisites, missing private billing metadata and a cancellation failure that passes isolated with process access.

Packaged DMG checksum VALID. Review verified all changed existing files are exact three-case name substitutions except .gitignore preservation rules, corrected AGENTS guide links, and README bootstrap transition note. Renamed Java Maven/Gradle integration tests pass. Full validation remains pending prerequisite exceptions; task stays In Progress.

Additional quality review: ignored local benchmark also scores 10 (278 reviewed files total). Root integration java_duplicates: two real Maven/Gradle cases pass; private recurring case requires unavailable CODECALLNAVIGATOR_RECURRING_BILLING. Java flow, HTTP and lambda correlation tests pass.

All 35 root integration targets completed: 210 tests passed, two failed. Private recurring duplicate case lacks fixture. TypeScript source-alias byte equivalence fails only analysis.inputs and limitation count; nodes/edges match, rename causality unproven. Recorded as unresolved validation; no unrelated semantic fix. Verification report: docs/verification/rename-2026-11-11.md. No commit or push. Task remains In Progress pending required validation exceptions/failure resolution.
<!-- SECTION:NOTES:END -->
