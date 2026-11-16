---
id: TASK-22.11
title: Present optional step-anchored AI flow explanations
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
updated_date: '2026-11-16T07:11:00Z'
labels: []
dependencies:
  - TASK-22.10
references:
  - src/flow_view.rs
  - src/flow_view/details.rs
  - src/flow_workspace.rs
  - src/ui_tests/function_flows.rs
  - docs/function-flowcharts.md
documentation:
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
parent_task_id: TASK-22
type: feature
ordinal: 22110
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add the user-facing explanation experience defined by the approved design, consuming validated anchored results. Generated prose must remain visually and semantically separate from analyzed control flow and preserve source navigation. Start with ordinary single-function charts; revised or expanded views must either have correctly bound anchors or an explicit unavailable state, never reused text from another chart.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Users explicitly request/cancel explanations under the approved provider/context flow; chart opening and target following do not automatically submit source.
- [ ] #2 Generated text is visibly labeled as interpretation and linked to selectable chart steps/source, with provider provenance and supported partial/unsupported-context notices.
- [ ] #3 Switching functions, generations, revisions or expansion state cannot display mismatched anchors or stale explanations; unsupported contexts are clearly unavailable.
- [ ] #4 Native acceptance covers step selection, cancellation, empty/partial output, provider errors, missing configuration and offline behavior at minimum-window light/dark layouts.
- [ ] #5 User documentation, full verification and source Code Health checks cover the delivered provider route and its limits; real provider evidence is distinguished from fixture/UI evidence.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
