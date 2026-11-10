---
id: TASK-30.2
title: Establish full-payload analysis equivalence corpus
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T13:52:00Z'
labels: []
dependencies:
  - TASK-30.1
parent_task_id: TASK-30
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Corpus covers all current languages, build tools, flows and architecture
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Extend complete persisted payload comparison; formalize real-language fixtures; verify reference runs and repeatability; record exact tools and limitations.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reference repeats and extracted engine now pass all five real-language workloads. Complete results in /private/tmp/codegraph-extraction-final.jsonl; optional-field and SQLite enum representation normalized without suppressing semantic differences. Eight focused harness tests pass; CodeScene safeguard passes.
<!-- SECTION:FINAL_SUMMARY:END -->
