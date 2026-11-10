---
id: TASK-29
title: Match connected nodes to the selected node level
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:14:00Z'
updated_date: '2026-11-10T13:22:00Z'
labels: []
dependencies: []
references:
  - TASK-23
type: bug
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up to TASK-23: focused class and folder graphs inherit mixed manual expansion levels, allowing broad groups to pull unrelated nodes and edges into the neighborhood. Use the selected semantic level consistently, including namespace/package focus, while preserving navigation and overview expansion.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Focused class, namespace/package, project, layer and method views contain only nodes at the selected semantic level and exclude unrelated branches.
- [x] #2 Folder focus uses selected build-relative directory depth with shallower and project-root folder fallbacks, including cross-project connections.
- [x] #3 Single-click focus and explicit uniform child drill-down preserve overview expansion, class methods, history, camera, filters and legacy session restoration.
- [x] #4 Shared typed projection policy separates hierarchy, representative construction, aggregation and interaction; regression tests, native visual evidence and CodeScene checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reproduce mixed-level class leakage with a failing projection regression. 2. Introduce typed focused-level resolution and shared hierarchy representative selection; aggregate before bounded traversal. 3. Add explicit persisted group-child drill-down preserving overview state and existing class-method focus. 4. Extend projection and native navigation regressions including namespace/package, folder depth, roots, layers and sessions. 5. Run Rust, Clippy, scoped formatting, Graphviz, native capture and CodeScene checks; record evidence and finalize.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer completed read-only review. Initial class focus regression fails as expected because a collapsed neighbor remains a folder. Sonar inactive: no .sonar/config.json.

Implementation complete: typed focused presentation policy and explicit GroupChildren focus preserve overview expansion. Screenshot regression now passes; namespace/package, folder depth/root, project, layer and method cases added. Reviewer found no production blockers; requested project/layer cases added and passed. Native class capture inspected: only Caller, OrderService and OrderRepository, two edges, unrelated Metrics omitted. CodeScene findings in test setup addressed via fixture extraction and combined navigation assertion; all 11 modified source files now score 10.0. Strict Clippy reports pre-existing warnings in unchanged viewport and benchmark tests; rerun with only those two lint classes exempted passes. Full suite caught old expansion expectation and asynchronous layout assertion; updated to approved behavior and wait for real layout.

Final verification: 301 Rust tests passed, 50 ignored; all integration targets completed. Separately enabled both packaged Java resource tests: first restricted attempt failed on Gradle cache lock, normal cache-access rerun passed 2/2 with real Maven/Gradle/JDK artifacts. Python suite passed 50 tests. Native dark/light captures inspected. Source provenance verification, scoped formatting and whitespace checks pass. CodeScene safeguard passed 11 eligible files with no issues. Evidence and logs: evidence/ui/aligned-levels/README.md. Strict Clippy remains limited by unchanged pre-existing warnings; only those two lint categories were exempted for the passing diagnostic rerun. No release bundle, installation, commit or push performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented uniform focused node levels, including namespace/package and selected-depth folder connections, through a typed projection policy and shared aggregation. Explicit persisted child drill-down preserves overview expansion and navigation compatibility. Regression tests, native dark/light rendering, independent review/testing, 301 Rust tests, 50 Python tests, 2 packaged-resource smoke tests and CodeScene checks passed. Strict Clippy still reports existing warnings in unchanged rendering/benchmark files; diagnostic rerun with those lint categories exempted passes. Development binary rebuilt; changes remain uncommitted.
<!-- SECTION:FINAL_SUMMARY:END -->
