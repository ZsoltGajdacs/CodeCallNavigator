---
id: TASK-56
title: Evaluate optional Codex desktop graph freshness reminder hooks
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T18:51:00Z'
labels: []
dependencies:
  - TASK-54
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
type: spike
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Optional experimental spike, dependent on the Codex skill package. Target manually installed desktop plugin hooks only per current packaging docs; reverify official surface restrictions first. In isolation, inspect PostToolUse apply_patch input for explicit project files, ignore unknown shapes/shell/MCP calls, and emit at most a 512-character advisory reminder per turn with session deduplication metadata only. No graph injection/cache/refresh, blocking or argument rewrite. Validate exact-definition trust, changed-definition re-review, disabled/untrusted hooks, repeats, failures, timeout and removal. Compare editing with and without reminders; record host version, event delivery, usefulness and overhead. Keep CLI plugin-hook support unclaimed unless separately proven. Full verification/CodeScene for retained executable prototype.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Normal desktop hook trust, changed-definition re-review, disable/removal and editing comparisons have evidence or precise unavailable validation records.
- [ ] #2 A release recommendation is evidence-backed and opt-in; public plugin hook distribution and unvalidated surface parity are not claimed.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recheck Codex desktop plugin hook distribution and trust constraints.
2. Prototype only isolated manual-desktop PostToolUse reminders using the task bounded policy.
3. Validate definition-hash trust/re-review, disablement, repeats, failures, timeout and removal under ordinary trust.
4. Compare edit journeys with and without reminders, report overhead and usefulness and recommend or reject release; run applicable full verification and CodeScene.
<!-- SECTION:PLAN:END -->
