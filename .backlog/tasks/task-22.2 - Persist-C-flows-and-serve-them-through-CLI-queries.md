---
id: TASK-22.2
title: Persist C# flows and serve them through CLI queries
status: To Do
assignee: []
created_date: '2026-11-14T11:49:00Z'
labels: []
dependencies:
  - TASK-22.1
references:
  - crates/codecallnavigator-engine/src/analyzers/csharp.rs
  - crates/codecallnavigator-engine/src/analyzers/csharp/ownership.rs
  - crates/codecallnavigator-core/src/flow/lookup.rs
  - crates/codecallnavigator-core/src/flow/inventory.rs
  - crates/codecallnavigator-core/src/query/operations.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22020
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Rust AtlasMap has no flow fields, normalize_atlas never fills GraphData.flow_inventory, and shared flow lookup explicitly permits only Java/TypeScript. Connect raw C# facts to the existing flow/source/query contract through the same build/assembly/path mapping as graph nodes. This delivers a headless C# chart without changing desktop presentation. Keep unsupported/ambiguous callables unavailable or partial with reasons; old artifacts remain usable for graph exploration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An actual C# project analyzed through the production CLI yields a generation-bound flow query with chart, exact cached source, stable callable identity and preserved target certainty/provenance.
- [ ] #2 Build, assembly, project and linked-source ownership are consistent across nodes, flow functions, calls, canonical artifact serialization, composition/pruning and SQLite round trips; duplicate or unmapped references are rejected.
- [ ] #3 Old saved graphs without C# flow metadata remain readable and return a refresh/unavailable explanation; missing or incompatible required fresh-analyzer capabilities reject publication and retain the previous generation.
- [ ] #4 Real CLI tests verify overloads, linked files in distinct compile contexts, generated/body-unavailable declarations, Unicode source ranges, and source retrieval after current files change or disappear.
- [ ] #5 Before/after graph-equivalence checks show no unrelated node, relationship, entry-point or existing Java/TypeScript-flow changes; opening a flow performs no analysis or compilation.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
