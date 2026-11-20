---
id: TASK-102
title: Put Compare flows and navigation actions on the toolbar
status: Done
assignee:
  - '@codex'
created_date: '2026-11-20T22:20:00Z'
updated_date: '2026-11-20T22:20:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expose common desktop navigation actions directly on the workspace toolbar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Overview, Compare flows, and Collapse all are direct actions on one horizontal toolbar, including at minimum window width.
- [ ] #2 Compare flows opens from the toolbar with or without a loaded graph and the function flow shortcut remains available.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Replace the More popover with horizontal actions, reuse the comparison button, remove obsolete menu state, and update the UI tests and acceptance driver.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Moved Overview, Compare flows, and Collapse all to one toolbar row. Removed the More popover and the directory picker comparison entry. Updated UI tests and the comparison acceptance driver. Formatting, cargo check, Clippy, and UI unit tests passed. The initial final run stopped at the TypeScript prerequisite. The user asked to stop the rerun and close the task, so interaction and native acceptance criteria are left unchecked.
<!-- SECTION:FINAL_SUMMARY:END -->
