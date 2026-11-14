---
id: TASK-22.10
title: Generate validated explanations anchored to analyzed flow steps
status: To Do
assignee: []
created_date: '2026-11-14T11:50:00Z'
labels: []
dependencies:
  - TASK-22.9
references:
  - crates/codecallnavigator-core/src/flow/model.rs
  - crates/codecallnavigator-core/src/query/contract.rs
  - src/flow_workspace.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22100
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the provider/execution boundary selected in the explanation decision. This task starts only after the user has resolved that design gate; dependency completion alone is not approval for an unspecified external provider. Generate interpretation over a bound analyzed chart, validate every returned anchor and preserve normal chart operation on service failure. Do not store model text as compiler facts or alter graph relationships.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The approved single provider/execution route accepts an explicit generation request bound to a specific artifact, function and chart identity, using only the approved context and configuration.
- [ ] #2 Responses validate against a bounded schema and existing step/source anchors; unknown anchors, changed generation/chart identity and oversize/malformed output are rejected or explicitly represented as validated partial results.
- [ ] #3 Every accepted explanation is labeled generated interpretation with model/provider provenance; compiler facts, canonical graph artifacts and native flow paths are unchanged.
- [ ] #4 Cancellation, deadlines, credential/configuration errors, rate/service failures and offline operation leave the underlying chart usable and never leak credentials into logs/artifacts.
- [ ] #5 Deterministic provider-boundary fixtures cover success and failure modes; record one explicit real-provider end-to-end check using approved context, or report that live integration gate as unavailable.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
