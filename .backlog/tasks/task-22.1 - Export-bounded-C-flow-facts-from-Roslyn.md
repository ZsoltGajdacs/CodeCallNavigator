---
id: TASK-22.1
title: Export bounded C# flow facts from Roslyn
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-14T11:57:00Z'
labels: []
dependencies:
  - TASK-21
references:
  - vendor/callatlas/src/CallAtlas.Core/CallGraphBuilder.cs
  - vendor/callatlas/src/CallAtlas.Core/CodeMap.cs
  - vendor/callatlas/src/CallAtlas.Core/CodeMapJson.cs
  - patches/README.md
  - analyzers.lock.json
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22010
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
C# analysis already holds Roslyn syntax, semantic models and assembly-qualified callable identities, but CallAtlas exports neither statement trees nor per-site flow targets. Add a versioned flow-facts capability to the existing extraction pass, without changing graph inclusion or native call binding. This is the raw exporter boundary; Rust ingestion and desktop enablement are separate children. The initial supported subset is synchronous structured bodies; unsupported evaluation/control constructs must become bounded opaque regions, not eager guessed calls. All upstream edits must be captured in the checked-in combined patch and bootstrap metadata.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Raw pinned-analyzer output has an explicit flow capability/version and deterministic per-callable facts for block/expression-bodied methods, explicit accessors, local functions and top-level bodies where the existing graph has an unambiguous identity.
- [ ] #2 Supported straight-line statements, if/else, ordinary loops, simple switches, return/throw and short-circuit/conditional expressions retain evaluation order; unsupported syntax produces explicit partial/opaque facts without exposing nested calls as unconditional.
- [ ] #3 Function and call-site spans map to exact original UTF-8 source bytes, including Unicode, CRLF and BOM inputs; call targets reuse compiler/graph identities and preserve declared targets, candidates and unresolved/reference distinctions.
- [ ] #4 Export depth/node budgets fail visibly at the callable boundary; callbacks and local-function declarations do not become eager execution merely through lexical nesting.
- [ ] #5 Real Roslyn fixtures cover overloads, duplicate type names across assemblies, same-line call sites and deferred bodies; enabling facts leaves existing nodes, edges, entries and unrelated diagnostics unchanged.
- [ ] #6 The recorded patch, base revision, checksum and exact-previous-patch upgrade remain reproducible; stale/incomplete capability output is distinguishable and existing upstream failures are compared with a baseline.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [ ] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->
