---
id: DRAFT-3
title: Explore near-real-time graph freshness during code edits
status: Draft
assignee: []
created_date: '2026-11-11T13:48:00Z'
updated_date: '2026-11-11T13:48:00Z'
labels:
  - exploration
  - freshness
  - performance
dependencies: []
references:
  - TASK-4
  - TASK-30.12
  - TASK-39
  - DRAFT-1
  - DRAFT-2
documentation:
  - docs/standalone-engine.md
  - docs/graph-contract.md
type: spike
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
While an agent or developer edits a codebase, both agent queries and the desktop graph viewer should reflect the current code with minimal delay. Explore fast change detection and graph refresh so consumers can tell which source state their graph represents and avoid silently relying on stale context.

Existing manual refresh, portable freshness verification and inference-decision caching provide starting points. Inference cache reuse does not by itself establish incremental compiler analysis or continuous refresh. This draft seeks a measured feasibility assessment and recommended scope, including what near-real-time can mean for supported languages and project sizes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The report defines measurable freshness and responsiveness targets for representative project sizes and supported languages, distinguishing saved filesystem changes from unsaved editor buffers.
- [ ] #2 Change-detection and refresh alternatives are compared with existing manual refresh, input verification and inference caching; compiler/build costs and safe incremental invalidation boundaries are identified.
- [ ] #3 Representative edits, rapid edit bursts, file additions/deletions/renames and build/configuration changes are assessed for detection-to-publication latency, resource use and correctness against full analysis, with unavailable measurements identified.
- [ ] #4 The report describes coherent graph generations and freshness states shared by desktop and agent consumers, including concurrent queries/edits, cancellation, invalid intermediate code and preservation of the last successful graph.
- [ ] #5 A recommendation records feasible latency expectations, language-specific limitations, open decisions and implementation follow-up tasks without promising freshness beyond measured capability.
<!-- AC:END -->
