---
id: TASK-103
title: Make Compare flow-first
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-22T19:11:00Z'
updated_date: '2026-11-23T04:18:00Z'
labels: []
dependencies: []
type: feature
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Open saved-snapshot comparison to a change outline and paired flow canvases, with unified source diffs on hover and equivalent CLI/MCP access.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Recognized entry points group changed classes and methods with ungrouped changes still visible.
- [ ] #2 Paired flows show attributable changes and unified diff previews from saved source on hover, focus, and class or file selection.
- [x] #3 Versioned paged CLI and MCP results agree, preserve snapshot binding and maintain existing comparison contracts.
- [ ] #4 Relevant deterministic final, native and CodeScene checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Derive compact changed entities and reachability in core with saved source unified hunks via diffy.
2. Expose bound list and detail in the engine and MCP with unchanged legacy contracts.
3. Add the native outline, default flow selection and floating previews.
4. Document and verify the public contracts and native journey.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented the snapshot-bound list and unified source diff across core, engine, MCP, and the native workspace. The first development run found a borrowed diff lifetime error. Switched the serialized hunk lines to owned data.
Development verification passed all seven stages in target/verification/changes-q7kriu4y after the lifetime fix. A full selector run will cover the subsequent ownership and native acceptance refinements.

Restored full-run prerequisites using an unlinked Graphviz 16.1.1 keg and the project fixture TypeScript installation. Verified in an isolated checkout because the existing desktop app was using the main checkout release directory.
Added exact-source step highlighting for changed call targets and certainty. The latest development run passed all seven stages in changes-takfb4q1. The corrected full run is in progress.
Earlier full integration evidence in changes-zwxwq_xb found a legacy source test that still assumed a docked pane by default. The test now enters Functions through its real control before checking the existing pane and provenance.

Final selector run changes-do05elzt passed 109 of 112 stages, including the unified diff headless render regression, real CLI and MCP parity, performance, CodeScene, and cleanup. Evidence is retained at target/verification/task-103-evidence/final/summary.json. Three native gates were deferred because macOS was locked, including the hover, keyboard focus and class preview replay. The earlier native run reached hover and exposed Rune requiring GrowthSpec on the virtual rows. The renderer now applies it and the regression proves inserted source renders. Acceptance criteria 2 and 4 remain open for native verification.
<!-- SECTION:NOTES:END -->
