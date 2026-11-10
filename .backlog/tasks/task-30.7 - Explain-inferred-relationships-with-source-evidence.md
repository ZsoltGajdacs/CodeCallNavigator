---
id: TASK-30.7
title: Explain inferred relationships with source evidence
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T14:43:00Z'
labels: []
dependencies:
  - TASK-30.6
parent_task_id: TASK-30
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Evidence survives export, storage and inspection without conflating invocation and references
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add versioned structured source evidence for inferred relationships; preserve through validation, canonicalization, composition, store and inspector; attach evidence to existing React static rules before adding new resolution. Verify old caches and full corpus equivalence except explained evidence additions.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Versioned structured relationship evidence now validates and survives export/hydration, composition/pruning, SQLite and the inspector. Existing React rules attach reference/deferred/possible semantics with UTF-8 source spans. Independent real-engine review proved full graph equals foundation after removing evidence; both corpus changes are evidence-only. Workspace tests, 50 JS tests, evidence/UI tests, strict Clippy and CodeScene pass. Three-run release TS median +1.6%, sampled RSS essentially unchanged; report evidence/engine/stages/evidence-performance.json.
<!-- SECTION:FINAL_SUMMARY:END -->
