---
id: TASK-37
title: Replace sidebar search with Double Shift search
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T07:41:00Z'
updated_date: '2026-11-11T08:08:00Z'
labels: []
dependencies: []
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Sidebar search consumes workspace space and lacks dedicated class results. Deliver the approved modal and focused maintainability refactoring.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Double Shift and Cmd/Ctrl+F open a keyboard-accessible modal above graph and flowchart views; sidebar search is removed.
- [x] #2 All, Classes and Methods provide deterministic case-insensitive substring search over the complete graph with pagination and disambiguation.
- [x] #3 Result activation reveals and focuses the correct method or class with one history entry; stale responses and legacy preferences are handled.
- [x] #4 Search ownership and shared identities are consolidated, with automated coverage and required quality checks recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add shared typed class identity and immutable core name index with matching regressions. 2. Consolidate transient search state, asynchronous jobs and stale-response checks. 3. Replace sidebar search with a common modal overlay and standalone Double Shift detector. 4. Centralize result activation and preserve navigation and saved preferences. 5. Update acceptance tests, verify layout, run Rust, packaging and CodeScene checks, and record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared typed class identity and immutable core name lookup; unified fixture/production matching; consolidated search workspace; added modal above graph and flowchart workspaces with Double Shift and toolbar/Cmd-F. Added recorded Rune input-observer patch because focused editors consume ordinary keys before the app handler. Reviewer findings fixed: stale-query activation guard, empty workspace state, layer-tab reveal, and synchronous query paging reset. Core tests and strict Clippy pass; desktop tests pass with built engine; Python suite 76 passes. CodeScene source reviews completed and safeguard passes (23 eligible files). Packaging succeeded. Running full compiler acceptance and release-only canvas checks; visual matrix underway.

Final validation: 178 normal desktop tests and all 11 modal regressions pass; core 46 tests pass; strict workspace Clippy passes. Full acceptance initially had five prerequisite failures, all resolved by executed reruns: three release benchmarks pass with copied cache; billing source acceptance passes after isolated Maven rebuild; compiled acceptance passes with refreshed fixture outputs (10827 declarations, 17331 relationships). Python 76 tests, patch migration 7 tests, and Rune observer 4 tests pass. Metal visual matrix inspected in both themes at both sizes, plus packaged real-time search smoke. Packaging app/ZIP succeeded. CodeScene safeguard passes all 23 eligible files. Sonar inactive because project configuration is absent. Evidence: evidence/ui/search-modal/README.md. Independent reviewer findings and tester-discovered category focus issue resolved; no unresolved findings.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced sidebar search with Double Shift/Cmd-F/toolbar modal across graph and flowchart views. Added shared substring name index, class/method tabs, keyboard navigation and generation-safe results. Consolidated search state and class identities; recorded Rune input-observer patch with migration coverage. All acceptance criteria verified; detailed validation and initial prerequisite failures with passing reruns are recorded in evidence/ui/search-modal/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
