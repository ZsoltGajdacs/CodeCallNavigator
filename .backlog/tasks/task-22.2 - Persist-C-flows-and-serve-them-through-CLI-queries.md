---
id: TASK-22.2
title: Persist C# flows and serve them through CLI queries
status: To Do
assignee: []
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-14T13:59:00Z'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-22.1 independent review identified a mandatory ingestion contract case: raw flow spans include actual UTF-8 BOM, while current CallAtlas ArchitectureFileFacts.Content uses tree.GetText().ToString() and strips BOM; csharp/architecture.rs currently persists it unchanged. Preserve the raw exact-byte span contract and bind to exact original source content during ingestion (with verified equality to compiler text after BOM handling), or reject mismatch explicitly. Add production CLI/persistence BOM+Unicode+CRLF regression; otherwise existing source query validation/highlighting would be offset by3bytes. This is not fixed or waived by raw exporter acceptance.

Compatibility checkpoint for implementation: distinguish owned UTF-8 compiler/disk mismatch (reject unsafe publication) from valid source encodings that the new exact-byte flow exporter does not support. Existing C# graph/source facts can be compiler-decoded from UTF-16 or other accepted encodings; do not make the new flow feature fail an otherwise supported graph-only analysis merely because fs::read_to_string cannot decode original bytes. Prove baseline behavior with a real UTF-16 source fixture, preserve its graph behavior, and make flow explicitly unavailable/unsupported with a reason if exact UTF-8 spans cannot be bound. Keep this policy in the source-binding boundary, not by parsing human diagnostic strings; test unsupported/generated/unowned paths alongside BOM-bearing UTF-8 success.

Shared-consumer audit: core query capabilities currently list Java/TypeScript; advertising C# truthfully will automatically enable the existing production desktop inspector action through its CLI capability lookup. Do not add a duplicate UI language gate or withhold a valid capability to preserve the old timing assumption. TASK-22.4 remains the native/package acceptance milestone; parent C# delivery stays incomplete until it passes. Fixture capability mocks can remain legacy examples. MCP result validation is language-agnostic and needs no graph logic. Update the explicit C#-unsupported claim in docs/cli-queries.md when CLI support lands. Shared source validation must preserve version-1 nested 0..0 spans as unlocated/synthetic sentinels, validate meaningful ranges against function/snapshot UTF-8 bounds (not immediate-parent lexical nesting), and keep the 16 MiB source-display limit at query time rather than turning it into graph-publication failure. Existing Java/TypeScript synthetic cleanup/bind/deferred-region spans and fixtures are regression inputs.
<!-- SECTION:NOTES:END -->
