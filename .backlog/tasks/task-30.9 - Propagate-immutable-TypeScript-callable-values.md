---
id: TASK-30.9
title: Propagate immutable TypeScript callable values
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T15:57:00Z'
labels: []
dependencies:
  - TASK-30.8
parent_task_id: TASK-30
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Logical fallbacks, parameters and returned functions share graph and flow targets
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
First correct the discovered export-identity collision in a separate commit: use exact registered AST node-to-index mappings, preserve native binding, and prove named identity stability through body/comment/position changes. Then freeze the observer-stage engine, add callable corpus source and commit its pre-propagation graph. Implement a strict callable-flow resolver alongside legacy bindings: immutable aliases, short-circuit logical values, direct parameter target convergence and per-call return environments. Share exact invocation descriptors/evidence with graph and flow. Defer mutation, spread/rest, registries and captured factories. Verify positive/negative real compiler tests, reviewed graph deltas, runtime/memory and CodeScene before finalizing.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Observer dependency completed in 00db2bc. During canonical review found that positional declaration lookup lets child modifier/name syntax overwrite export keys; correcting exact export declaration ownership first is required by the approved portable-identity contract.

Portable-identity prerequisite fixed using an exact registered AST node-to-index WeakMap for export only; native positional lookup unchanged. Regression first failed with missing module identity, then passed for module/declaration kinds, overload/ownership distinctions and named body/comment/movement stability. Complete frozen-Stage2 versus candidate native graph JSON is identical after removing only export_identities; all 79 JS tests pass. Canonical TypeScript output is repeat/relocation byte-identical. CodeScene reviews 10.0 and safeguard passed.

Established pre-propagation fixture with frozen corrected observer engine3a261b6: 49 nodes,36 edges,4 existing observer edges and no callable propagation edges. Covers logical nullish/or aliases, immutable aliases, actual parameter invocation, per-call identity returns and forwarding/mutation negative controls. Fixture CodeScene10.0, pre-commit safeguard passed.
<!-- SECTION:NOTES:END -->
