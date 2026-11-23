---
id: TASK-106
title: Make the Compare outline Git-first
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-23T18:36:00Z'
updated_date: '2026-11-23T20:38:00Z'
labels: []
dependencies: []
type: feature
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compiler-generated lambda IDs can differ between HEAD and Worktree without a source edit. Use Git to determine changed paths and source hunks, with saved graphs supplying optional attribution and flow details.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Exact commit and Worktree comparisons show each changed path once, even without saved graphs.
- [x] #2 Class and method rows require changed lines and matching saved source, so compiler-only lambda churn does not produce changes.
- [x] #3 Paging and file previews reject changed Worktree bindings and Reload reads Git without analysis.
- [x] #4 Desktop, CLI, and MCP share the source comparison while saved-graph APIs and Impact Preview keep their behavior.
- [ ] #5 Real Git and artifact regression cases, native Compare, and required verification are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Share Git path and text-diff primitives and capture the effective Worktree or exact commit endpoints.
2. Attribute actual changed ranges to matching saved spans.
3. Wire the versioned source query through CLI, MCP and desktop with independently bound paging and previews.
4. Verify source-only behavior, lambda churn, Worktree races, and native class or method drill-down.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created from the approved Git-first plan. Reused Git path and blob handling, source hunk rendering, and saved graph ownership. Review reproduced the index deletion plus recreation case and embedded repository directory markers. The first development runs exposed core ownership moves and desktop type mismatches, now addressed in the working changes.

The packaged engine returned 29 changed paths and zero AccrualDbService rows for the original Recurring Billing HEAD versus Worktree comparison while loading both saved graphs. Development, Git capture and attribution, CLI/MCP parity, desktop reload and cleanup, native Compare in both themes, performance, and CodeScene passed. Final selector run changes-n2qcubvd passed 106 stages and deferred the general C# desktop gate because macOS was locked. That gate passed in changes-08a8jt3h, immediately before the CodeScene-only graph-loading helper extraction. Acceptance of the earlier result is pending.
<!-- SECTION:NOTES:END -->
