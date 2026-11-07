---
id: TASK-2
title: Implement persistent graph storage and queries
status: Done
assignee:
  - '@codex'
created_date: '2026-11-07T22:00:00Z'
updated_date: '2026-11-07T22:27:00Z'
labels: []
dependencies:
  - TASK-1
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Keep large analyzed codebases searchable and traversable without drawing the whole graph; persist provenance and diagnostics so cached results retain their meaning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SQLite persists normalized project graphs diagnostics and searchable symbols
- [x] #2 Adjacency queries cycles shortest paths and filters retain complete graph access
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Define normalized dense-index graph types with engine provenance and unavailable body references. 2. Persist immutable generations in SQLite with atomic active generation publication and FTS. 3. Build adjacency queries, bounded neighborhoods, shortest paths, cycles and filters. 4. Verify cancellation, failed refresh retention, roundtrip metadata and deterministic graph algorithms with real SQLite.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented typed graph model, CSR adjacency, shared filters, bounded neighborhoods, shortest paths and SCC cycles. Real SQLite immutable generations with atomic catalog pointer, FTS and sessions. Four behavioral tests pass including failed/cancelled/empty refresh retention and omitted-node reachability. Independent source review completed; modelCodeHealth10, graph9.38, store9.68, tests10. Core full-scale performance verified separately in TASK-5.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added complete-graph storage and queries with explicit body-unavailable references and preserved provenance. Real SQLite and graph behavior tests pass; data survives rejected refreshes.
<!-- SECTION:FINAL_SUMMARY:END -->
