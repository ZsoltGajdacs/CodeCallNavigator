---
id: TASK-4
title: Add graph refresh and session recovery
status: Done
assignee:
  - '@codex'
created_date: '2026-11-07T22:00:00Z'
updated_date: '2026-11-07T23:17:00Z'
labels: []
dependencies:
  - TASK-2
  - TASK-3
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Preserve the last successful graph and user navigation when projects change or an analysis is cancelled or fails.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Manual atomic refresh cancellation stale detection and cached session restoration preserve the last successful graph
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Verify immutable-generation publication and cancellation, source/build freshness, generation-pinned queries and exact-identity session recovery in the implemented background lifecycle.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented atomic refresh with last-successful preservation, descendant process cancellation, source/build fingerprints and manual stale refresh. Generation-pinned search avoids publication races; corrupt sessions recover while cached graphs open. SQLite lifecycle, freshness, process cancellation and UI restoration tests all pass.
<!-- SECTION:FINAL_SUMMARY:END -->
