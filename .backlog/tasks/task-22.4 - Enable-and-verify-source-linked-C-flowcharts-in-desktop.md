---
id: TASK-22.4
title: Enable and verify source-linked C# flowcharts in desktop
status: To Do
assignee: []
created_date: '2026-11-14T11:49:00Z'
labels: []
dependencies:
  - TASK-22.3
references:
  - src/flow_workspace.rs
  - src/inspector.rs
  - src/flow_view/details.rs
  - src/ui_tests/function_flows.rs
  - docs/function-flowcharts.md
  - evidence/flowcharts/README.md
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22040
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The desktop already consumes CLI-built charts and owns layout, navigation and source highlighting. Finish C# availability and user-facing limitations through this path, without duplicating flow construction in the UI. This is the first complete user-visible milestone for TASK-22 and must be independently shippable before explanations, comparisons or recursive inlining.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Selecting a supported C# method in a real analyzed workspace opens its cached-source-linked chart through the installed CLI; step selection highlights the correct source and target navigation distinguishes overloads and uncertainty.
- [ ] #2 Native acceptance verifies target navigation and Back, independent camera/scroll history, stale-generation rejection, older-cache refresh guidance and minimum-window layout.
- [ ] #3 Partial/unsupported C# behavior and absent bodies are explained visibly; unavailable actions do not imply a complete chart is available.
- [ ] #4 Packaged/relocated analyzer and Graphviz resources produce a real C# chart; record refresh/artifact overhead and selected-chart latency on named fixtures without benchmarking the live application store.
- [ ] #5 Supported C# semantics and limitations are documented; full verification, change-specific light/dark native evidence and source Code Health checks are recorded with unavailable gates or baseline failures identified.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
