---
id: TASK-69
title: Split compressed graph artifacts and lazily release function flows
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T11:51:00Z'
updated_date: '2026-11-16T15:21:00Z'
labels: []
dependencies: []
type: feature
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Recurring-billing produces a 122 MB compact graph, 69 percent function flows that ordinary navigation does not need. Implement the approved clean-cut two-part compressed format and staged residency on current branch codex/development without legacy compatibility.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Deterministic compressed manifest and objects publish atomically with safe retention and Git loading.
- [x] #2 Ordinary queries do not load flow bodies; shared typed flow access serves normal expanded and historical flows.
- [x] #3 Generation-bound CLI load unload and status control residency and safe recovery.
- [x] #4 UI releases hidden flow inventory and presentation caches after 60 seconds with safe reopening.
- [x] #5 Contracts fixtures docs inventory relevant verification CodeScene and performance evidence match the cutover; broader verification limitations are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Separate structural indexes and typed flow access. 2. Add Zstandard object manifest publication retention Git loading. 3. Add CLI flow residency and clean version cutover. 4. Add UI visibility deadline and cache release. 5. Migrate fixtures docs inventory and verify boundaries UI performance full suite CodeScene.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented slices in progress: separate structural GraphIndex ownership, shared typed flow view, deterministic compressed bundle and generation capture, explicit CLI residency, UI 60-second release. Recorded Python verification-only BLAKE3/Zstandard dependencies for independent bundle schema and fault-fixture tooling. Current branch remains codex/development. Agents own isolated verification workspaces.

Focused verification passed: core 158 and engine 63 tests, workspace compile, UI unit category, tooling unit category and independent bundle/schema checks. Same-session corruption recovery and old-generation unload integration tests pass. CodeScene safeguard checked 131 eligible files with no regressions. Full runs 1-2 exposed Clippy findings now fixed; full run 3 is active. Benchmark uses a disposable source copy because original recurring-billing graph disappeared during this session; live checkout remains untouched.

Current copied-source recurring-billing graph measured 123467818 monolithic bytes versus 7203807 bundle bytes (structure1961024 flows5242444 plus manifest). Real CLI session starts structure-only; load/unload/reload succeeds and every unload reports zero retained flow allocations. Debug reload timing about5.5-5.9s is explicitly not compared to installed prior release latency; RSS can retain allocator pages. Full Git comparison on two v2 fixture commits succeeds, debug28.29s and maximum RSS2310307840bytes. Evidence target/verification/task69-measurements. Full run5 passed unit categories and is checking real compiler integration.

Release engine reused from full verification measured five reload cycles: median875.35ms maximum899.83ms; first flow1148.74ms including freshness/chart response. Unloads remain idempotent and report zero retained ownership. RSS settles above initial baseline due allocator retention, so this is not represented as immediate physical-memory reclamation. Evidence release-residency.json. Found and fixed comparison success envelope still hardcoded version1; now uses query::VERSION and integration helper asserts envelope version.

Remaining root fixture readers now resolve bundle objects while retaining exact manifest-byte assertions; fourteen targeted TypeScript pipeline/source-alias/incremental-inference acceptance tests and seven comparison tests pass. Full6 uses the installed standard TypeScript6.0.3/React19/Next16 fixture set; the earlier6.0.2 wrapper delegates default libraries under @typescript/old and is not recognized by the existing fetch detector. Analyzer behavior was not changed. Compact Rust debug information is disabled for this run to fit disk capacity; debug assertions remain enabled and category-selected profiles are unchanged. Full6 is active.

Full6 reached UI integration after compiler-backed C#/Java/TypeScript suites passed; eleven old-fixture ownership/envelope failures were reproduced and corrected without weakening assertions. Focused UI category now passes231 UI integration and11 real CLI/compiler journeys including idle unload/reopen. Final CodeScene safeguard passes140eligible files with no regressions; waiting category cleanup before full7.

Full7 passed all unit categories and root/core/engine UI/compiler integration, including recurring-billing; MCP integration exposed two raw analysis fault-probe hellos still using obsolete versions. Corrected valid hello to2 and invalid probe to999; CodeScene process.rs10. Focused MCP integration is running before the next full pass; no production behavior or assertions changed.

Focused MCP integration passes with corrected protocol probes; cleanup succeeds. Read-only acceptance audit found no remaining outer-v1 or manifest-only fixture assumptions. Also corrected missing-fields wire probe to v2 so it cannot pass solely on wrong-version rejection. CodeScene both MCP files10. Full8 launched for final combined gates.

Full8 all Rust integration passed, including MCP probes; Python266tests exposed11 installer fixture errors from old query_version1. Correcting installer test fixtures. Required acceptance/performance gates will now run by category before another composite retry, avoiding repeated compiler runs for fixture-only failures. No required gate is waived.

Performance category passed all native/query/storage gates and cleanup: target/verification/task69-performance-1. Acceptance1 packaging/signature/MCP-install passed but C# input8tests used one old manifest-as-graph helper; migrated to artifact::load, preserved raw manifest recovery assertions, CodeScene10. Installer14 focused tests pass. Acceptance2 now running.

Acceptance2 passed all Rust acceptance, automatic refresh, analyzer cleanup, native C# desktop and flow ingestion. Native comparison timed out Editing(Entry) before external operations; retained report/fixture under task69-acceptance-2. Investigating native UI event step without weakening timeout/assertions. Tooling integration rerun now checks installer fixture correction; performance remains passed.

Installed CLI21 operations, standalone CLI agent20operations and real MCP19operations passed. Native comparison timeout was a stale driver clicking Compare flows before opening project picker; driver now follows actual picker controls with a new regression test, CodeScene10 and safeguard145eligible no regressions. Isolated signed native package built; comparison and expansion native gates scheduled sequentially. Wrapper recheck passes after inventory registration completed.

Native comparison corrected driver passes26checks in each theme with six screenshots; real-runtime entry regression passes. Standalone cleanup hit transient inconclusive lsof then fresh audit proved owned fixture inactive; agent is completing recorded cleanup/retry. Final full9 composite started after source/inventory freeze; includes installer fixtures and native-driver correction.

Full9 passed units/root/compiler/UI integration, then a pre-existing fault-probe marker race read an empty PID in flow_comparison cancellation test. This left owned processes and correctly failed category cleanup. Fixing atomic PID publication and auditing only that owned fixture/process before reclaiming retained build. Native comparison/expansion behavior passed separately; acceptance3 now runs category-owned cleanup as the conclusive gate. No timeouts/assertions or cleanup checks weakened.

User directed completion with relevant checks only. Stopped the newly started broad integration rerun; its category cleanup passed with zero retained bytes. No further broad or environment-only reruns will be pursued. Relevant core/engine/UI/MCP tests, installer and CLI/MCP journeys, native comparison/expansion functional checks, performance category and final CodeScene safeguard145eligible pass. Full composite is explicitly NOT claimed clean: earlier runs exposed corrected fixture/probe issues, standalone native cleanup inspection was inconclusive before fresh proven-idle cleanup, and latest native category stopped on AeroSpace window setup before interaction. AC5 narrowed from full-composite completion to the user-directed relevant verification scope; all limitations remain in evidence and notes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented clean-cut v2 manifest plus deterministic Zstandard structure/flow objects, atomic publication and Git reads, structural-only startup, explicit generation-bound CLI residency, and UI release after60seconds hidden. Refactored shared typed flow access and ownership; migrated contracts fixtures docs and tooling. Recurring-billing current analysis123467818bytes to7203807bytes(94.2% smaller), median release reload875ms. Relevant verification and CodeScene pass; broad/environment limitations recorded, no full-composite pass claimed. Current branch codex/development retained.
<!-- SECTION:FINAL_SUMMARY:END -->
