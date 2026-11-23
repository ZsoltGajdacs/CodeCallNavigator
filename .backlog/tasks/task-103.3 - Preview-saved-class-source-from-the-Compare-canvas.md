---
id: TASK-103.3
title: Preview saved class source from the Compare canvas
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T13:48:00Z'
updated_date: '2026-11-23T17:08:00Z'
labels: []
dependencies:
  - TASK-103.2
parent_task_id: TASK-103
type: feature
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move class source inspection to the canvas. Hover and focus should retrieve source from the compared revisions and avoid attributing a shared file to the wrong class.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hovering or focusing a class node shows an attributable diff, a saved excerpt, or an explicit unavailable state.
- [x] #2 A selection or snapshot change prevents late previews from appearing.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Resolve class source from saved ownership and flow spans; add a bound on-demand CLI query and reuse the existing renderer.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added a dedicated CLI query used by the native preview controller. Diffs use saved file ownership or exact member spans, with bounded excerpts or an unavailable state. An initial shared-file test rejected its fixture's missing architecture facts version. The fixture now records the current version.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Class canvas hover and keyboard focus show an attributable saved diff or excerpt. Selection changes invalidate pending requests and clear the preview. Source ownership tests and both native themes passed. Final evidence: target/verification/changes-mdea3q11/summary.json.
<!-- SECTION:FINAL_SUMMARY:END -->
