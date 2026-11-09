---
id: TASK-19
title: Add an independent virtual and interface calls filter
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T15:10:00Z'
updated_date: '2026-11-09T15:15:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Discovered Java virtual and interface calls are hidden with other uncertain targets. The reported RecurringResourceImpl.updateRemark to BatchUpdateService.updateRemark edge should be independently visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Independent Virtual / interface calls checkbox with compact label and help controls dispatch edges without changing other uncertainty, references or rendering.
- [x] #2 New and legacy saved filters enable virtual calls by default; explicit settings persist through history and reload.
- [x] #3 Traversal, paths, grouped projections and hidden counts obey the new setting; saved endpoint graph verified.
- [x] #4 Behavioral tests and required Code Health checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add backward-compatible include_virtual_calls and shared edge filtering. 2. Wire checkbox, help, legend and details. 3. Cover combinations, serialization, history, traversal and projection. 4. Validate saved graph, Rust tests and Code Health; finalize task.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner and explorer reviewed invariants, compatibility and validation routes. Added behavioral regression and observed expected failure before implementation. Confirmed only root AGENTS applies; Sonar inactive because .sonar/config.json is absent. Copied active saved graph to temporary cache for endpoint verification without reanalysis or user-cache mutation.

Focused regression checks pass. External verification against a copy of the active saved graph passes: RecurringResourceImpl.updateRemark to BatchUpdateService.updateRemark remains VIR/dispatch, appears with Virtual calls enabled and Uncertain disabled, and is removed from neighbors, shortest path and neighborhood when Virtual calls is disabled. No project reanalysis or user-cache writes performed.

Implementation complete: default-on backward-compatible virtual calls filter, independent shared edge policy, descriptive labels/help/legend. UI geometry tests reproduced clipping, fixed by bounded wrapping and compact labels below 1150px. Regression test failed before fix; 30 focused integration tests passed; full Rust suite passed 160 tests with 36 explicitly ignored toolchain acceptance cases; 51 UI tests passed after helper extraction and added relationship-detail test passed. Independent reviewer found no issues. All 10 modified Rust files reviewed at Code Health 10.0 without findings; final safeguard passed. Clippy across all targets with warnings denied and formatting checks passed. Sonar inactive (no project configuration).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added independent default-on Virtual / interface calls filter, including older saved views. Shared traversal/projection/filter summaries obey the new setting while preserving stored dispatch and dashed rendering. Descriptive UI controls wrap on narrow canvases. Verified exact saved endpoint service connection with virtual calls on/off and Uncertain off without reanalysis. Rust, UI, Clippy, formatting, independent review and Code Health checks passed; toolchain-specific ignored tests were not run.
<!-- SECTION:FINAL_SUMMARY:END -->
