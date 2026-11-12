---
id: TASK-55
title: Evaluate optional Claude Code graph freshness reminder hooks
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T18:51:00Z'
labels: []
dependencies:
  - TASK-53
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
type: spike
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Optional experimental spike, dependent on the Claude skill package. In an isolated configuration, implement a disposable PostToolUse reminder for explicit Edit/Write paths only; ignore unknown shell commands and MCP calls to prevent recursion. Emit at most a 512-character advisory reminder per turn for edited project files; do not block/rewrite tools or query/analyze graph automatically. Keep per-session deduplication metadata only, no graph cache. Validate real interactive hook trust, disablement, repeated edits, failures, removal and timeout. Compare the same editing journey with and without reminder, record delivery timing and model freshness checks, and recommend release only with measurable benefit. Run full verification/CodeScene if executable prototype is retained.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reproducible comparison records usefulness and overhead plus disabled/untrusted/failing-hook behavior under normal Claude trust.
- [ ] #2 The report either recommends a separate opt-in artifact with evidence or rejects hooks; no production hook is installed.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recheck Claude hook documentation and create an isolated optional reminder prototype with the bounded policy in the task description.
2. Validate ordinary trust, disabled/untrusted/failed hooks, repeat suppression, timeout and removal.
3. Compare identical edit journeys with and without hooks and record overhead and observable freshness-check behavior.
4. Recommend opt-in release only with demonstrated benefit; otherwise reject hooks. Apply full verification and CodeScene to retained source.
<!-- SECTION:PLAN:END -->
