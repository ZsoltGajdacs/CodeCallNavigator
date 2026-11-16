---
id: TASK-22.9
title: Define the optional flow explanation provider and data contract
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
updated_date: '2026-11-16T07:11:00Z'
labels: []
dependencies:
  - TASK-21
references:
  - crates/codecallnavigator-core/src/flow/model.rs
  - crates/codecallnavigator-core/src/query/contract.rs
  - src/flow_workspace.rs
  - docs/function-flowcharts.md
documentation:
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
parent_task_id: TASK-22
type: spike
ordinal: 22090
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-22 requires optional AI interpretations, but the codebase has no explanation backend, provider configuration or credential flow. A design decision is needed before adding networking or assuming source may be sent externally. This spike produces a reviewable specification and bounded implementation handoff, not production provider code. Keep compiler/source facts distinct from generated interpretation and keep ordinary chart use fully functional without AI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A short decision document presents viable provider/execution choices and a recommended single initial route, identifying provider/model selection, configuration and credential ownership, cancellation/timeouts, request/response budgets and testability.
- [ ] #2 The proposal specifies exactly which cached source/chart context may leave the machine, how a user initiates generation, what is previewed/disclosed, and retention/cache/logging behavior; open product decisions are explicitly listed for user review.
- [ ] #3 Explanation anchors bind to artifact generation/digest, function and step/source identity, with rules for duplicated cleanup steps, partial flows, stale results and future revision/expanded views.
- [ ] #4 A bounded response contract separates generated interpretation from compiler facts, rejects unknown anchors and defines partial responses, retry/failure/offline states and deterministic test fixtures.
- [ ] #5 The decision and unresolved choices are recorded for explicit review before provider implementation; follow-on generation/UI tasks are updated to the approved scope without changing the parent acceptance criteria.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Verify document references, open decisions, scope and diff; run ./verify-light.sh in documentation-only mode.
- [ ] #2 Keep unresolved implementation choices explicit and update dependent task boundaries after decision review.
<!-- DOD:END -->
