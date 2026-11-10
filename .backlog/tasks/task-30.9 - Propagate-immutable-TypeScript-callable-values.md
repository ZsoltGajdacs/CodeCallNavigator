---
id: TASK-30.9
title: Propagate immutable TypeScript callable values
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T16:32:00Z'
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
- [x] #1 Logical fallbacks, parameters and returned functions share graph and flow targets
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

Final Stage3 verification: 90 real-compiler TypeScript tests and11 targeted propagation cases passed. Independent review confirmed exact10 new invocation edges,49nodes46edges in TS and75nodes61edges in mixed corpus; all preexisting nodes/edges including observers unchanged. Graph/flow targets, certainty, provenance and portable evidence agree. Rust2flow+9pipeline tests passed; canonical repeat/relocation byte equality passed. Explicit-undefined defaults and optional-factory skipped-call errors were reproduced then conservatively deferred. Final3-run median .542348708s baseline vs .562072125s candidate (+3.64%); sampledRSS367886336 vs367542272bytes. Report evidence/engine/stages/callable-performance.json. All modified JS reviews10.0; pre-commit safeguard passed11eligible files.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented strict immutable logical aliases, finite direct parameter propagation and call-specific returned functions with shared graph/flow evidence. Preserved native binding, React maps, references and observer targets. Positive, negative, relocation, pipeline and independent review checks passed; documented conservative unsupported cases. Export-identity correction and fixture baseline were committed separately before propagation. Local commits only; no push.
<!-- SECTION:FINAL_SUMMARY:END -->
