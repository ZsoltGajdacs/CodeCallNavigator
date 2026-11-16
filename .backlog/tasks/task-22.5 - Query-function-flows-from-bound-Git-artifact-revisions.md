---
id: TASK-22.5
title: Query function flows from bound Git artifact revisions
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-16T07:11:00Z'
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
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
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
- [x] #1 A versioned public CLI operation returns selected before/after functions, charts and analyzed source bound to explicit endpoint refs/commit IDs, artifact digests and a comparison identity.
- [x] #2 Exact stable identity is the automatic correspondence rule; additions/removals, missing flow/source and incompatible artifacts have explicit per-side outcomes, and renamed/moved methods are never silently matched.
- [x] #3 Results use each endpoint's cached source and portable ownership mapping even when live source differs or is absent; old artifacts lacking source/flow report that limitation without recompilation.
- [x] #4 Real Git/CLI fixtures cover two commits, commit-versus-WORKTREE, added/removed functions, moved refs or changed artifact bytes, and mismatch rejection across subsequent bound requests.
- [x] #5 Requests/results are bounded and cancellable; tests prove no checkout, fetch, refresh, source writes or canonical/cache mutation, and existing compare/query-session contracts remain compatible.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Implement and independently verify the portable saved-flow view against strict validated artifacts, with unique node/function/source ownership and exact cached-source attribution. Share ordinary Flow availability and chart validation; preserve legacy output and avoid graph hydration.
2. Separate revision inspection from strict decoding while retaining requested refs, resolved commits, paths, exact-byte digests and observed schema. Factor endpoint pairing without changing existing Compare identities or error order.
3. Publish optional version-1 flow_comparison catalog/detail requests. Use exact identity by default, explicit labeled manual counterparts, per-side availability and unknown membership for incompatible artifacts.
4. Bound requests, whole-record catalog pages, source bytes and the complete response envelope. Keep Git reads cancellable and read-only; preserve existing query/session behavior.
5. Verify real Git/CLI cases for committed endpoints and WORKTREE, missing live/linked source, changed refs/artifacts, ownership ambiguity, manual pairing, exact byte boundaries and cancellation. Review/refactor each implementation unit.
6. Integrate onto feature/task-22-function-flowcharts after TASK-22.4 completion, document the public contract, run CodeScene and full verification outside the sandbox with isolated profiling inputs, then close only against concrete evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer current-code audit complete. All choices remain approvedhistorical-flow scope. PortableGraph validation alone does not enforce function/node ownership or duplicatefunction/source tuples; consumedfactsmustcheck explicitly. Root4 full-qiaiih4v continues againstfrozen source, andnew5sourceis confined to /private/tmp/codecallnavigator-task22.5 detachedworktree untilintegration. Backlogrecord remainsoriginalworkspace; no separatetaskmetadatawritesfromtemporarycheckout.

Integrated reviewed units onto feature branch after TASK-22.4 completed at 5df0173. Desktop unlocked and native minimum-size light/dark acceptance passed; earlier pending-native notes are historical. Public API and ownership/revision refactors are complete, with 163 focused passing tests before independent edge additions. Root full verification remains pending; do not treat isolated reports as full acceptance.

Root integration and independent review complete. Focused final audit: 168 tests passed, one existing SDK-dependent test ignored, core/engine Clippy passed; five additional public CLI edge tests found no production defect. Both final test files reviewed at Code Health 10 by MCP; installed CLI integration gate passed before full. Full run target/verification/full-k7cvs5hs is active (62 stages), source frozen; no full-pass claim until terminal. Durable focused evidence: evidence/flowcharts/revisions/.

Full verification completed successfully: target/verification/full-k7cvs5hs, 62/62 stages passed. Final independent review approved all five added boundary tests. Root source stayed unchanged during the run; only task metadata was refreshed. Final MCP safeguard passed with 35 eligible files and no issues. Durable full results, source hashes, focused reports and quality evidence are recorded under evidence/flowcharts/revisions/.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added version-1 flow_comparison catalog/detail queries for saved Git artifacts. Results retain endpoint refs, commits, exact-byte digests, cached source and call-target provenance; exact identity is automatic and explicit counterparts are labeled manual. Shared strict inspection/ownership/chart validation preserves existing Flow and Compare behavior without hydration. Independent focused audit passed 168 tests (one existing SDK test excluded only from focused scope); full-k7cvs5hs passed all 62 stages, including all ignored release acceptance, real compiler fixtures, benchmarks, packaging/signatures, installed CLI/MCP journeys and cleanup. Both CodeScene versions passed; MCP safeguard checked 35 eligible files with no issues. Evidence: evidence/flowcharts/revisions/README.md. Sonar inactive (no project configuration). No PR/push requested; desktop presentation follows in TASK-22.6.
<!-- SECTION:FINAL_SUMMARY:END -->
