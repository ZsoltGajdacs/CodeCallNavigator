---
id: TASK-22.3
title: Validate C# cleanup and conservative control-flow boundaries
status: To Do
assignee: []
created_date: '2026-11-14T11:49:00Z'
labels: []
dependencies:
  - TASK-22.2
references:
  - crates/codecallnavigator-core/src/flow/model.rs
  - crates/codecallnavigator-core/src/flow/builder/exceptions.rs
  - crates/codecallnavigator-core/src/flow/builder/loops.rs
  - tests/flow_core.rs
  - tests/java_flows.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22030
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Existing shared cleanup machinery was built for Java/TypeScript. C# using statements/declarations, foreach disposal and lock release need correct scope-specific unwinding; catch filters run before stack unwinding, and iterator yield is not Java switch yield. Extend only the cross-language primitives required for a documented synchronous subset, with language extraction in the adapter. Complex unsupported cases must remain opaque/partial. Do not claim complete C# runtime semantics or invent async scheduling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Published support matrix distinguishes exact supported structure, conservative exceptional paths and opaque/unavailable constructs for try/catch/finally, using statements/declarations, foreach, lock, catch filters, guarded patterns, goto, await and iterator yield.
- [ ] #2 Real .NET marker traces for nested using scopes, acquisition failure, early return, break/continue, throw/rethrow and throwing finally match chart paths; cleanup occurs only for acquired resources and on the correct exits, without impossible fallthrough.
- [ ] #3 Synchronous supported foreach/lock cases preserve acquisition/evaluation and disposal/release order on normal and abrupt exits; unsupported custom/async variants are diagnosed instead of flattened into misleading ordinary actions.
- [ ] #4 Filtered catches never appear as unconditional catch-all handlers and retain possible uncaught paths; unsupported guarded/goto transfers are visibly partial and do not fabricate exact destinations.
- [ ] #5 Async/iterator bodies and deferred callbacks are not represented as certain eager execution; C# yield is never decoded using Java switch-completion semantics.
- [ ] #6 Shared model changes preserve old artifact behavior or introduce explicit compatible version handling, with existing Java/TypeScript cleanup, switch and callback regressions passing.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
