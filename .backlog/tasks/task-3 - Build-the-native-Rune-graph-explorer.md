---
id: TASK-3
title: Build the native Rune graph explorer
status: Done
assignee:
  - '@codex'
created_date: '2026-11-07T22:00:00Z'
updated_date: '2026-11-07T23:17:00Z'
labels: []
dependencies:
  - TASK-1
  - TASK-2
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provide a graph-only Rune interface for hierarchy and method-level exploration, including global-namespace codebases and large repositories.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Native Apple Silicon app supports hierarchy search graph navigation filters paths and inspector
- [x] #2 Background layout and bounded canvas remain responsive with omitted counts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Native Rune window with root/build selection and background jobs. 2. Render Graphviz geometry with hierarchy/focus/path projections and1500/6000 paging. 3. Search across full graph, inspector, directional controls, filters, history and camera. 4. Headless interaction tests and screenshots, then live macOS verification.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the native Rune graph explorer: qualified hierarchy and folder grouping, full indexed search, bounded asynchronous Graphviz canvas, method neighborhoods, callers/callees, paths, cycles, filters, provenance and navigation. All 12 UI tests and 5 projection tests pass; final light/dark real-project GPU screenshots reviewed. Live hardware validation is tracked separately in TASK-5.
<!-- SECTION:FINAL_SUMMARY:END -->
