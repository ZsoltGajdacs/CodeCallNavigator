---
id: TASK-22.8
title: Expose optional flow expansion through CLI and desktop
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
labels: []
dependencies:
  - TASK-22.7
references:
  - crates/codecallnavigator-core/src/query/contract.rs
  - crates/codecallnavigator-core/src/query/operations.rs
  - src/flow_workspace.rs
  - src/flow_view/details.rs
  - src/ui_tests/function_flows.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22080
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Publish the shared expansion result through the CLI and consume it in desktop interaction. Keep expansion decisions and source ownership in the CLI/core; UI owns expand/collapse state, layout and navigation. The existing single-function Flow request remains the default behavior and reference for collapsed output. An MCP tool is not required to deliver this desktop capability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An opt-in versioned bounded CLI request produces an expanded chart with source/occurrence ownership; invalid paths, stale generations and unsupported targets fail explicitly, while legacy single-function requests retain their behavior.
- [ ] #2 Desktop supports expand/collapse at a selected call site and explicit candidate selection, shows method boundaries and cycle/limit reasons, and keeps source highlighting tied to the selected occurrence's file.
- [ ] #3 Expansion is cancellable background work; obsolete replies cannot replace another function/generation, and collapse restores the equivalent unexpanded flow.
- [ ] #4 CLI subprocess and native UI fixtures verify multi-file expansion, repeated callees, recursive cycles, budget exhaustion, stale replies and Back/camera/scroll behavior.
- [ ] #5 Documentation and full verification record expansion bounds and semantic limitations; source Code Health, layout/performance measurements and change-specific visual acceptance accompany delivery.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
