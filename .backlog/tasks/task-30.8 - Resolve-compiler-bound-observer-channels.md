---
id: TASK-30.8
title: Resolve compiler-bound observer channels
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T15:51:00Z'
labels: []
dependencies:
  - TASK-30.7
parent_task_id: TASK-30
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Registration and actual collection invocation connect only related receivers
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
First add observer corpus source and commit its graph using the evidence-stage engine. Then recognize exact built-in Set collections, direct registrars and actual callback invocations. Support module const, private static readonly and proven closed-singleton private readonly fields; leave ambiguous receivers/mutations unresolved. Share invocation-specific targets between graph and flow; record registration/invocation evidence; verify negative controls and graph-only deltas.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Real TypeScript/Rust pipeline checks passed: two flow tests and nine pipeline tests; process-observation test rerun outside sandbox after /bin/ps permission denial. Independent review confirmed four expected corpus edges and found receiver safety gaps for structurally erased aliases, rebound dispatchers and inherited static private fields; conservative guards and regression reproductions are being completed before acceptance.

Final verification: 78 TypeScript tests passed; independent reviewer reran all 28 observer cases and actual standalone fixture. TS and mixed graphs add exactly four codegraph.observer.v1 edges; all pre-existing nodes/edges are byte-equivalent. Graph/flow symbols, certainty and provenance agree. Final three-run median 0.525893s baseline versus 0.526584s candidate (+0.13%); sampled tree RSS 324829184 versus 321945600 bytes. Report: evidence/engine/stages/observer-performance.json. CodeScene source reviews 10.0; pre-commit safeguard passed for 16 eligible files. Sonar remains inactive (no project configuration).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Resolved proven built-in Set observer channels with registration/invocation evidence shared by graph and flow. Conservative receiver, mutation, escape and decorator guards keep unrelated storage unresolved. Real corpus and independent negative controls passed; no compiler-bound edge changes. Committed locally as the next ordered resolution stage; no push.
<!-- SECTION:FINAL_SUMMARY:END -->
