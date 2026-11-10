---
id: TASK-30
title: Extract standalone graph engine and improve resolution against Git baselines
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:41:00Z'
updated_date: '2026-11-10T20:02:00Z'
labels: []
dependencies: []
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved roadmap: preserve merged baseline, extract independent engine, publish one project-local canonical graph and compare Git versions, then precise inference stages after equivalence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Foundation verified against master fb54f5f before resolution changes
- [x] #2 One deterministic .codegraph/graph.json per project and Git comparator
- [x] #3 Desktop invokes packaged standalone engine
- [x] #4 Resolution stages verified in dependency order
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Complete foundation 0A-0F in separate commits, verifying frozen native payload equivalence before inferred relationships. Then implement evidence, observer channels, callable propagation, framework dispatch, HTTP/navigation and incremental inference.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Completed all 12 foundation and resolution tasks in dependency order.

Local master preserves immutable baseline fb54f5f789d94c6cbc5fe86bf20010edea815b33. The foundation gate at e23420c passed before inference work. Subsequent work remains on codex/standalone-graph-engine; nothing was pushed and remote master was not modified.

Delivered codegraph-core, independently buildable codegraph-engine, desktop subprocess integration, one atomic canonical graph per project, portable identities, complete graph/flow/architecture export and direct Git comparison. Existing language/build adapters and checked-in upstream patch bootstrap are preserved. The engine does not require Rune, WGPU or Graphviz.

Resolution checkpoints: evidence 91e666f; observers 00db2bc; portable identity correction 3a261b6; callable propagation a22e651; Spring 008575b; MediatR b7207e2; navigation/HTTP 3f8b652; incremental decisions b839ba3. Fixtures were frozen separately before behavior changes, with real compiler positive and negative controls. Upstream adaptations and attribution are documented in docs/engine-adaptations.md.

Final verification: all seven packaged corpus graphs are byte-identical to 3f8b652. Cold/warm/reordered/relocated output agrees. Fourteen SQLite tests and three real CLI tests cover dependency invalidation, full equality, corruption and failure/cancellation preservation. Workspace all-target tests, strict Clippy, CodeScene safeguards and signed app/standalone packaging pass. Controlled final runtime medians stay within 1.5%; raw JVM/RSS variability is retained and explained.

Limits: only pure boundary decisions are cached; compiler extraction and earlier inference remain fresh/full. Dynamic, ambiguous or unsupported wiring remains unresolved as documented. C# flow inventories remain unavailable. CallAtlas has 262 passing tests and the same 29 preexisting failures; all five new HTTP cases pass. Private/GPU/toolchain exclusions remain explicitly documented rather than claimed as passing.

Usage and architecture: docs/standalone-engine.md. Consolidated evidence: evidence/engine/README.md, with linked boundary and incremental reports. Built artifacts: dist/CodeGraphEngine/bin/codegraph-engine and dist/CodeGraph.app. Matching frozen distributions record their exact source checkpoints for rebuilding and comparison.
<!-- SECTION:NOTES:END -->
