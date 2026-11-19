---
id: TASK-98
title: Confirm build targets before manual desktop analysis
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19T10:51:00Z'
updated_date: '2026-11-19T12:04:00Z'
labels: []
dependencies: []
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Review and update build targets before Generate graph or Refresh starts a manual run. Auto Analysis keeps its direct controls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generate graph and Refresh show the current target selection in a confirmation dialog.
- [x] #2 Confirmation saves and analyzes exactly the reviewed targets. Dismissal discards the draft.
- [x] #3 Analysis settings and Auto Analysis retain their existing behavior.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reuse the picker with overlay-owned draft state and shared selection bookkeeping. Exercise production Rune controls and a real multi-target journey. Review Code Health and run the deterministic selector.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented the approved desktop-only manual confirmation with shared picker rendering, overlay-owned draft identities, one selection-change notification, and existing manual scheduling. Independent source review found no production blockers. The initial selector exposed a CodeScene finding in the new Rune test; simplifying its control flow resolved the finding. Final selector target/verification/changes-nzb3ug03 passed 42 of 45 stages, with CodeScene at 10.0 for all six reviewed Rust files and the safeguard passing. Rune controls, the real Maven/TypeScript target and SQLite persistence journey, existing Auto Analysis journeys, packaging/signature, benchmarks and cleanup passed. Three packaged C# and flow desktop journeys were deferred by the macOS console-session preflight, so the task remains In Progress. Independently opened and cancelled the new modal in a PID-verified native 1050×680 window and inspected target/verification/task-98-visual/native-manual-analysis-confirmation.png. Final selector timings in seconds: setup 19.7, compile 426.4, test 81.8, cleanup 28.0, other 120.4.

The user explicitly accepted the task-specific native 1050×680 check as sufficient and approved completion with the following three packaged checks deferred: acceptance-gate-csharp_desktop_acceptance, acceptance-gate-flow_comparison_desktop_acceptance, and acceptance-gate-flow_expansion_desktop_acceptance. The selector did not run them because the macOS console session could not be verified. See target/verification/changes-nzb3ug03/summary.json.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Generate graph and Refresh reuse the Build Targets picker for manual confirmation. Start analysis persists the reviewed targets and passes them to the existing manual action. Cancel discards draft changes. Analysis settings and Auto Analysis retain their existing controls. The selector passed 42 of 45 stages, including the real mixed-build desktop journey and CodeScene. The user accepted the independently verified native 1050×680 check in place of the three recorded deferred packaged journeys.
<!-- SECTION:FINAL_SUMMARY:END -->
