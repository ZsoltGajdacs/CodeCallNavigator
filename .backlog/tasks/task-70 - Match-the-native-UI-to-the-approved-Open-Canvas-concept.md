---
id: TASK-70
title: Match the native UI to the approved Open Canvas concept
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T15:49:00Z'
updated_date: '2026-11-16T16:36:00Z'
labels:
  - ui
  - ux
dependencies: []
references:
  - TASK-68
documentation:
  - doc-34
type: enhancement
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow up TASK-68 visual fidelity gaps shown by user screenshots. Approved full pass preserves curved routes and graph workflows. User requires current branch and UI-only verification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Toolbar, Files tree and iconography match approved concept while preserving actions and shortcuts.
- [x] #2 Compact activity and graph controls preserve real lifecycle status, diagnostics and responsive access.
- [x] #3 Cards, inspector, filters and source previews follow approved styling with all real content and behavior.
- [x] #4 Presentation owners and shared sizing tokens remain cohesive with aligned geometry.
- [x] #5 UI-only unit, integration, acceptance, performance and native comparison verification, CodeScene and cleanup evidence are recorded.
- [x] #6 All work remains on codex/development without a new branch or worktree.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Establish native references. 2. Refactor toolbar, activity and card presentation with shared tokens. 3. Match toolbar/tree and compact floating surfaces. 4. Match cards/inspector/filters/source previews. 5. Review and verify UI only, CodeScene and cleanup; update doc-34.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner and explorer reviewed current owners. Baseline offscreen Metal captures use existing release binary under target/verification/open-canvas-fidelity-baseline. Visual gaps are screenshot-proven; baseline method fixture has no real explorer root, so tree coverage uses real-filesystem UI journey. Implementation splits navigation, activity presentation and card rendering; shares card sizing. User override: UI tests only, current codex/development branch; no full/light, analyzer/core/MCP/tooling or packaging suites.

First unit build exposed standalone projection-test module linkage and incorrect Rune imports; fixed by owning shared card metrics under projection and correcting public icon/search imports. Fixed UI unit category passed (compile48.046s, setup2.089s, tests3.736s, cleanup1.869s). Independent review restored neighbor listbox keyboard/selection semantics and bounded long neighbor/source titles; added a real Rune keyboard regression. CodeScene modified source reviews pass10.0; native fidelity integration next.

Native physical-window captures succeeded in light/dark at1420x900 and1050x680 with owned PID/window leases and reaping (initial evidence under open-canvas-fidelity-native). Visual review corrected neutral raised/card palette, paired inspector actions and blue source link. Real Graphviz boundary assertion updated from old117.2px height to248x140 for approved header plus two member rows. Integration recheck passed234 runtime tests; real-CLI group exposed two tests that must open new activity details before asserting cache/freshness actions, now migrated. Fixed actual2px filter clipping and bounded picker width. Source previews now start440x260 and inspector pin records displayed node anchor before layout instead of default top-left. Diagnostic integration build retained temporarily (~1.28GB) for exact rechecks, scheduled for owned cleanup.

Final-source UI runtime group passes234 tests and native Metal fixture renders all requested states. Two remaining real-CLI test failures shared one cause: test navigation opened project actions while the new activity modal remained open; repeated per-test adjustments exposed this ownership gap. Centralized modal surface dismissal in the shared UI interaction helper using the real Escape route, preserving cache/freshness assertions and production behavior. Resized-preview overlap recovery now explicitly sets its large720x480 scenario instead of depending on the former default.

Exact final UI integration257/257 and UI unit219/219 passed under target/verification/open-canvas-fidelity-validation. Final warm compile50.776s, integration execution42.111s, unit replay1.159s. Unit replay used the already compiled release artifact to verify final disclosure/default changes after the earlier debug run, avoiding another unit rebuild; this is a final-source verification reason, not ordinary duplicate profile testing. Native comparison:45 offscreen Metal captures plus5 physical-window idle captures at1420/1050/light1128; real hover/source/status/filter interactions remain offscreen integration evidence. No material visual blocker. CodeScene safeguard passed34 eligible source files; Sonar unconfigured. Retained integration cleanup passed, reclaiming1,276,696,735bytes with zero retained build/temp; copied-native-binary cleanup reclaimed82,257,376bytes. UI performance3-test suite is running. Packaged acceptance/full/light/unrelated-component suites intentionally not run because they introduce excluded packaging or non-UI verification; focused native visual acceptance is recorded instead.

UI performance3/3 passed under target/verification/open-canvas-fidelity-performance: setup/snapshot2.318s, compilation133.587s, execution6.538s, cleanup1.727s. Dense Metal GPU-complete medians9.533/13.873ms at1x/2x and zero intermediate targets/extra passes; saved3.553/6.760ms. Current throughput only, no matched-baseline/FPS claim. Performance cleanup reclaimed1,208,617,578bytes with zero owned build/profile/temp retained; live profile source untouched. Final verification summary/action evidence: target/verification/open-canvas-fidelity-validation/README.md and verification-summary.json. All approved fidelity acceptance criteria now have behavioral/native/quality evidence; no packaging or push performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Matched the approved Open Canvas hierarchy: icon Files tree, toolbar/breadcrumb/search, circular compact activity and separate graph controls, neutral cards, inspector/source actions, grouped filters and compact anchored source previews. Refactored navigation/activity/card presentation with shared Graphviz/render sizing. Preserved curved routes and real graph workflows. UI unit219/219, integration257/257 and release performance3/3 passed; native45 offscreen states and5 physical idle-window captures reviewed. CodeScene safeguard passed34 source files; owned build/profile/temp and native copies cleaned. Stayed on codex/development. Packaging/full/light/non-UI suites excluded per approved scope; physical interactive/window pacing not certified.
<!-- SECTION:FINAL_SUMMARY:END -->
