---
id: TASK-30.12
title: Recompute inferred relationships incrementally
status: Done
assignee:
  - codex
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T19:57:00Z'
labels: []
dependencies:
  - TASK-30.11
parent_task_id: TASK-30
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Incremental output equals full analysis and stale inference is removed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Start only from verified Stage5 commit3f8b652 and frozen matching distribution. Always extract fresh compiler/base facts; earlier observer/callable/framework inference continues conservative full recomputation. Cache pure boundary decisions per use, never graphs/edges/flow/evidence. Keys include automatic rule-code/format version, usage, complete current in-scope candidate/blocker membership, config/toolchain and fresh source/target support; replay rebinds current identities and always builds evidence/flow from fresh facts. Reuse unaffected decisions; invalidate on matching route addition/removal/constraints/wiring or support changes. Keep only current rows in disposable project-local inference.sqlite3, exact-excluded with SQLite sidecars from discovery/fingerprints and Git. Missing/corrupt/incompatible cache falls back to full inference. Hold graph lock; verify inputs and atomically publish complete graph before replacing cache rows in one transaction; cache failures remain volatile reports. Provide CODEGRAPH_DISABLE_INFERENCE_CACHE=1 verification mode with identical canonical bytes. Test positive/negative invalidation, unrelated reuse, source-ID/index rebinding, cache corruption/version, disabled/full equality, cancellation/publication failure and no refresh loops. Compare candidate against frozen3f8b652 complete corpus, real changed-source runs, package app+engine, CodeScene/compiler/acceptance checks and isolated performance measurements.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Completed per-use pure boundary decision cache with automatic code/format invalidation, complete candidate/blocker and negative-membership dependencies, fresh identity/evidence/flow reconstruction and current-row-only SQLite persistence. Always fresh compiler facts and conservative full recomputation for earlier rules. Cache/sidecars exact-excluded from fingerprints; cache reads immutable; unowned/corrupt/link paths preserved; write errors volatile, transaction only after canonical graph publication.14 realSQLite tests and3 realCLI tests pass incl cached/full equality through edits/add/delete/config/wiring, currentindex rebinding, corruption, cancellation and failedpublication preservinggraph+cache. All7 packaged corpus graph files byte-identical to3f8b652; warm13/13hits, relocated/reordered equality. Workspaceall-target/strictClippy,CodeScene and signedapp/engine packaging pass. Controlled4workload medians within1.5%, no runtime regression above5%; both measurement rounds retained for JVM RSS variability. Evidence evidence/engine/stages/incremental.md. Earlier29upstreamCallAtlasbaseline failures unchanged; no vendor changes inthisstage.
<!-- SECTION:NOTES:END -->
