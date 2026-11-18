---
id: TASK-89
title: Show analysis failures and offer a retry with remaining builds
status: Done
assignee:
  - '@codex'
created_date: '2026-11-18T18:04:00Z'
updated_date: '2026-11-18T19:10:00Z'
labels: []
dependencies: []
type: bug
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A mixed Maven and TypeScript analysis failed on TypeScript and the empty desktop hid the diagnostic and recovery path. Preserve atomic publication and expose an explicit manual retry with the remaining selection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Manual and continuous failures report matching target and phase when attributable, preserving failure codes and prior graphs.
- [x] #2 Desktop shows the diagnostic with and without a graph and supports a validated manual retry excluding the failed target.
- [x] #3 Real mixed-build recovery and relevant desktop states pass deterministic verification.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Share analysis failure classification and carry target identity, phase, diagnostic and logs through manual and continuous transports. 2. Share bound desktop failure presentation and reuse existing selection and manual recovery actions. 3. Verify a real mixed-build failure and searchable Java retry plus desktop state transitions.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented both approved slices against b8315e1. Manual and continuous analysis use one engine failure classifier; optional build identity, phase and log directory are preserved in both transports. Desktop recovery uses structured, selection-bound state and existing manual handoff. Initial selector run changes-f3t9677i passed 38 stages including Cargo, Clippy and unit checks, then stopped for two missing fixture environment paths. Retrying complete selected plan with pinned TypeScript fixtures and an isolated Recurring Billing clone in changes-8ovxi8iv.

Changes 8ovxi8iv and xg9_y4m3 exposed a graphless input lock and a stale overlay in test setup. Moved recovery outside the inert graph world and exercised the automatic session via its action. Exact production run changes-nmfubk7t passed all 75 runtime, native, performance and cleanup stages. The new mixed journey passed in 10.64 seconds. The late CodeScene gate flagged a test helper. Extracted its unchanged visibility assertion and reran the CodeScene review and 23-source safeguard successfully. No production changes followed the passing runtime run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Manual and continuous analysis share attributed, bounded diagnostics. The desktop presents a graphless failure or preserves the previous graph and offers a validated retry with the remaining builds. The real mixed JVM and TypeScript journey confirmed a successful searchable Java graph after clicking the recovery button, with visible actions at narrow and wide sizes. Full selected runtime evidence: target/verification/changes-nmfubk7t/summary.json. Final quality evidence: target/verification/changes-nmfubk7t/codescene-after-test-refactor.json.
<!-- SECTION:FINAL_SUMMARY:END -->
