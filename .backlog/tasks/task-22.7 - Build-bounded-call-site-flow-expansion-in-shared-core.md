---
id: TASK-22.7
title: Build bounded call-site flow expansion in shared core
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
updated_date: '2026-11-14T11:51:00Z'
labels: []
dependencies:
  - TASK-21
references:
  - crates/codecallnavigator-core/src/flow/model.rs
  - crates/codecallnavigator-core/src/flow/builder.rs
  - crates/codecallnavigator-core/src/flow/lookup.rs
  - src/flow_view/details.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22070
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Today target following opens a separate function chart. Recursive expansion requires a distinct call-site-aware structure: FlowStep IDs are chart-local, and the same method may appear at multiple sites or in a cycle. Add a shared expansion result while preserving original function identities and per-occurrence ownership. Treat this as explicitly requested source exploration; never merge ambiguous dispatch alternatives or deferred callbacks into one guaranteed runtime path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Expansion identifies each occurrence by its call-site path while retaining original callable identity, owning source snapshot, source span, target relationship kind and resolution provenance.
- [ ] #2 A selected synchronous target can expand with explicit call/return boundaries; the continuation respects supported completion semantics, and unsupported invocation forms remain marked instead of being spliced as exact execution.
- [ ] #3 Direct/mutual recursion is detected on the active expansion path; repeated nonrecursive calls expand independently, and ambiguous/virtual targets require explicit candidate selection or remain unexpanded.
- [ ] #4 Depth, total step/link and source budgets are validated before work and produce visible cycle/limit/unavailable markers, with deterministic results and no silent truncation.
- [ ] #5 Unit fixtures cover recursion, diamonds, repeated calls, multiple calls within one expression, external/missing bodies and exceptions/cleanup; callbacks, references, navigation/HTTP targets and async bodies never imply certain synchronous invocation.
- [ ] #6 Original graph/flow inventories are unchanged by expansion, and existing single-function build output and legacy serialized artifacts remain compatible.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
