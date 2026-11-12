---
id: TASK-58
title: Expose Git graph comparison through CLI queries and MCP
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T21:48:00Z'
updated_date: '2026-11-12T22:29:00Z'
labels: []
dependencies: []
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers need explicit graph differences against Git history without checking out revisions or ingesting complete graphs. Extend the legacy comparison foundation with full changes and snapshot-bound pagination; automatic refresh and UI are excluded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI query and MCP compare saved WORKTREE or committed endpoints with resolved identities, digests and correct freshness.
- [x] #2 Deterministic full graph and metadata changes preserve exact identities, uncertainty, ordered flow and opaque extensions.
- [x] #3 Bounded pagination is complete and rejects changed snapshots and oversized records explicitly.
- [x] #4 Legacy compare behavior and existing CLI/session/MCP contracts remain compatible after focused refactoring.
- [x] #5 Real Git and MCP acceptance, full verification, CodeScene and large-graph measurements are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Separate snapshot retrieval from pure comparison and preserve legacy wrappers. 2. Implement typed full change records, ordering and bounded snapshot-bound pages. 3. Share request dispatch and expose compare in CLI query/session/capabilities. 4. Add thin MCP tool and operation-specific metadata validation. 5. Document, exercise real Git/MCP regression scenarios, review maintainability and run full verification plus large-graph measurements.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded approved initiative and plan. Added failing CLI comparison regressions before implementation (unknown query variant), then implemented portable snapshot retrieval, full typed differences, shared one-shot/session dispatch and thin MCP tool. Focused CLI and real stdio comparison acceptance pass. Review found graph.extensions metadata collision and partial-clone lazy fetch; fixed with explicit scope and no-fetch Git flags plus regressions. All 70 core tests and strict core/engine/MCP Clippy pass. Initial full preflight failed only for unset fixture/profile inputs; discovered existing paths and preparing rerun. Sonar is inactive: .sonar/config.json absent.

Independent reviewer confirmed fixes and found no remaining actionable issues. Independent tester added/passed two real-process Git-read cancellation/disconnect tests outside sandbox; CodeScene 10.0. All new/modified source files reviewed with CodeScene; findings resolved to 10.0. Pre-commit safeguard passed (26 eligible files at that point); final full CodeScene stage remains pending. Release benchmark on retained 113,628,316-byte compiler graph: 10,827 nodes, 17,331 edges; controlled label+diagnostic edits yield 2 records/7,492-byte response, median 4.3998 seconds and peak 4,017,881,088 bytes over three runs. Cost and stateless rereads documented in evidence/graph-comparison. Full rerun target/verification/task58-full passed preflight, formatting, bootstrap, compilation, MCP boundary and workspace Clippy; Rust suite running.

Final full run PASSED all 52 stages in target/verification/task58-full: 568 ordinary Rust tests, 661 release tests (zero ignored), 215 Python and 112 TypeScript tests, CPU/GPU and storage benchmarks, packaging/signature, installed CLI, independent MCP and agent journeys, CodeScene and cleanup. Private npm registry DNS warnings delayed builds but both private regressions passed; no waiver needed. All affected sources CodeScene 10.0; safeguard passed with 26 eligible files/no regressions. Added evidence/graph-comparison/README.md and latency-memory.json; source graph pair remains ignored under target/verification/task58-large-comparison. No UI changes or automatic refresh added.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented versioned compare queries and compare_graphs MCP with full scoped changes, exact-ID matching, stateless bounded snapshot-bound pages and target freshness. Refactored snapshot retrieval, shared CLI dispatch and MCP metadata validation while preserving legacy compare. All full verification gates and independent review/testing pass. Large retained compiler graph: median 4.3998s and peak 3.74 GiB; canonical copies and stateless recomputation are documented costs.
<!-- SECTION:FINAL_SUMMARY:END -->
