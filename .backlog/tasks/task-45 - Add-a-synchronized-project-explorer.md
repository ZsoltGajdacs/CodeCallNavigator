---
id: TASK-45
title: Add a synchronized project explorer
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T07:43:00Z'
updated_date: '2026-11-12T08:12:00Z'
labels: []
dependencies: []
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add the approved lazy all-file explorer synchronized with graph selection while retaining graph focus.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Lazy all-file tree supports loading, errors, retry, refresh and safe symlinks
- [x] #2 Graph selections reveal and highlight all represented files without stealing focus
- [x] #3 Single-click selects the explorer row; double-click frames filtered file classes with history
- [x] #4 Root races, persistence, UI regressions and required quality checks are verified
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Isolate explorer workspace, loader and view. 2. Centralize file matching and selection synchronization. 3. Add FileClasses projection and reusable focus bounds. 4. Test filesystem and Rune UI behavior, document and capture evidence. 5. Review and run Rust and CodeScene validation.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented lazy explorer workspace/loader/view and FileClasses projection, centralized selection resolution and focus bounds. Focused projection tests and default all-target tests passed. Independent review found same-node re-selection and lazy reveal ordering cases; fixes underway. Initial CodeScene review requested small extractions in explorer model/selection/view. Full include-ignored debug run stopped on release-only canvas benchmarks and detached engine child cleanup failure; strict clippy fixes underway.

Independent review and runtime testing caught and fixed repeated graph re-selection, lazy reveal ordering, Rune pointer-focus precedence, minimum-window filter occlusion, and canonical-source versus selected-root aliases. Final source ownership separates filesystem loader, selection resolver, tree state and view. Eight model regressions and seven real-runtime scenarios cover the feature; native dark1420/light1050 captures inspected. Required root tests, check, strict Clippy, Python106 and local packaging passed. Release CPU/GPU benchmarks passed using a transactional copy of the saved workspace. Available compiler acceptance and core/engine checks passed; exporting fresh private build metadata for the remaining compiled-output acceptance check. Evidence: evidence/ui/project-explorer-2026-11-12/README.md.

Final verification completed: private recurring-billing integration passed, fresh Maven test-compile metadata exported successfully, and the compiled-output acceptance regression passed. All compiler/core/engine tests completed through the documented split debug/release runs; sandbox cleanup failure passed on unrestricted retry. Final default root suite: 364 passed with70 ignored by default; all ignored scenarios were exercised in the appropriate acceptance/release runs. Final check, strict Clippy, CodeScene22-file safeguard, Python106, inspected native captures, and rebuilt local app package passed. No outstanding independent review or tester findings. No upstream edits or new dependencies.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the lazy all-file Project Explorer with independent scroll, refresh/retry and safe symlink handling. Graph selections reveal all represented files without stealing focus; file double-clicks frame existing classes with filters and history preserved. Centralized selection/path mapping and focus bounds, including selected-root aliases and minimum-window filter occlusion. Verified with8 model tests,7 real Rune runtime tests, full compiler/core/engine acceptance, release CPU/GPU checks, Python106, packaging and CodeScene. Evidence and command outcomes: evidence/ui/project-explorer-2026-11-12/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
