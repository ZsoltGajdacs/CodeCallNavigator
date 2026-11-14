---
id: TASK-22.6
title: Display before and after function flows with revision provenance
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
labels: []
dependencies:
  - TASK-22.5
references:
  - src/flow_workspace.rs
  - src/flow_view.rs
  - src/engine_client/queries.rs
  - src/graph_queries.rs
  - docs/function-flowcharts.md
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22060
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users need to inspect how a function changed across the base/head revisions of a review, rather than reading raw flow-inventory diff records. Consume the new revision-bound CLI flow results in a dedicated desktop comparison state. Use local refs that correspond to PR revisions; hosted PR discovery, authentication and fetching are outside this bounded delivery. If renamed methods need comparison, the user chooses both identities explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Desktop lets users choose base/head local revisions and a function, then displays before/after source-linked charts with visible requested refs, resolved commits, artifact provenance and partial-status notices.
- [ ] #2 Added/removed/unavailable sides are explicit; an optional manually chosen counterpart stays visibly manual and never becomes heuristic matching in the shared comparison.
- [ ] #3 Step selection and source highlighting remain local to the correct revision; target following, Back and switching comparisons cannot mix graph generations or endpoint sources.
- [ ] #4 Native acceptance covers changed/unchanged/added/removed functions, missing historical source, changed refs, cancellation and minimum-window light/dark layouts using real Git artifacts.
- [ ] #5 Documentation explains how local base/head refs represent PR comparisons and why missing historical graph metadata requires explicit preparation; no hosted-service integration is implied.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
