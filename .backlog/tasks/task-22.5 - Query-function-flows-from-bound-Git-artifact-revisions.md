---
id: TASK-22.5
title: Query function flows from bound Git artifact revisions
status: To Do
assignee: []
created_date: '2026-11-14T11:49:00Z'
labels: []
dependencies:
  - TASK-58
references:
  - crates/codecallnavigator-core/src/artifact/revision.rs
  - crates/codecallnavigator-core/src/artifact/difference.rs
  - crates/codecallnavigator-engine/src/entrypoints/comparison.rs
  - crates/codecallnavigator-core/src/query/contract.rs
  - crates/codecallnavigator-core/src/query/operations.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22050
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Git comparison already loads and validates saved artifacts without checkout and compares flow inventories by exact function symbol, but the Flow query only uses the current loaded graph. Add the bounded CLI contract needed to obtain before/after charts and their analyzed source from explicit saved-artifact endpoints. This is independent of C# export and can be verified on existing Java/TypeScript fixtures; no remote PR service or automatic historical analysis is part of this task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A versioned public CLI operation returns selected before/after functions, charts and analyzed source bound to explicit endpoint refs/commit IDs, artifact digests and a comparison identity.
- [ ] #2 Exact stable identity is the automatic correspondence rule; additions/removals, missing flow/source and incompatible artifacts have explicit per-side outcomes, and renamed/moved methods are never silently matched.
- [ ] #3 Results use each endpoint's cached source and portable ownership mapping even when live source differs or is absent; old artifacts lacking source/flow report that limitation without recompilation.
- [ ] #4 Real Git/CLI fixtures cover two commits, commit-versus-WORKTREE, added/removed functions, moved refs or changed artifact bytes, and mismatch rejection across subsequent bound requests.
- [ ] #5 Requests/results are bounded and cancellable; tests prove no checkout, fetch, refresh, source writes or canonical/cache mutation, and existing compare/query-session contracts remain compatible.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
