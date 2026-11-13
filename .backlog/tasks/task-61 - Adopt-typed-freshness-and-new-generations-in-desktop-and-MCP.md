---
id: TASK-61
title: Adopt typed freshness and new generations in desktop and MCP
status: Done
assignee:
  - '@codex'
created_date: '2026-11-13T08:05:00Z'
updated_date: '2026-11-13T14:47:00Z'
labels: []
dependencies:
  - TASK-60
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 69000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers currently lose unverifiable/failure context through a boolean, and desktop graph installation resets navigation and auto-fits. Consume the CLI observation contract and split background generation adoption from opening a workspace; MCP remains an independent CLI wire adapter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Desktop distinguishes matching, stale, unverifiable and failed observations without blocking the UI or duplicating engine verification.
- [x] #2 External artifact publication causes explicit generation reload; valid selected symbols, navigation history and camera survive while deleted selections and obsolete asynchronous jobs are discarded.
- [x] #3 Opening another project retains explicit workspace reset behavior; linked-root and project bindings cannot leak between generations.
- [x] #4 MCP forwards negotiated generation-bound observations and errors without graph algorithms, graph caches or watcher ownership; legacy CLI compatibility failures are explicit.
- [x] #5 Real desktop visual journeys, MCP subprocess acceptance, full verification and CodeScene cover adoption and generation mismatches.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consume optional observation-v1 capabilities through a small desktop capability record and an owned asynchronous observe transport. Validate bounded frames, versions, ordering, generations and liveness; preserve ordinary older-CLI graph queries with an explicit continuous-freshness unsupported state.
2. Add a focused freshness workspace bound to the loaded QueryContext descriptor, never the editable folder field. Preserve typed match/stale/unverifiable plus separate checking/unsupported/failed states. No desktop verification or scheduling; retry terminal failures explicitly.
3. Offer explicit Reload for a newer published generation. Load that exact generation asynchronously using the existing snapshot/cache path; tag work by binding/epoch/requested generation and discard superseded work. Investigate and prevent obsolete retention/session side effects alongside this first reload consumer.
4. Extract one graph installation operation with explicit restore/new-workspace/same-workspace adoption modes. Same-project refresh/reload preserves valid selections, all navigation/history/focus variants and camera; project changes reset. Reconcile through existing presentation focus rules, invalidate obsolete jobs and suppress late layout auto-fit/centering.
5. Add independent MCP graph_freshness tool mapping only to the negotiated one-shot CLI query. Validate independent wire DTOs and envelope generation, retain legacy summaries/status, and return explicit unsupported-capability errors for older CLIs. No core/engine dependency, artifact access, graph cache or watcher.
6. Add red-first real CLI/Rune desktop journeys and actual GPU screenshots for stale/unverifiable/reload/adopted states, old-job/generation races, deleted focus, edited folder and workspace changes. Add real MCP stdio parity/mismatch/legacy-capability/error checks, plus narrow transport/state tests.
7. Independent review/testing at coherent checkpoints, inspect rendered images, scoped formatting, CodeScene and full verification; finalize evidence and commit on the current branch before TASK-62.

8. Refine the first reload consumer boundary: CLI-validated snapshots own display state; derived profiling retention becomes one active/latest-pending write-only worker returning a marker, never a session writer. Only UI-accepted binding/generation/epoch may install markers and serialize the latest session/opaque settings. Retention failure is contextual and does not invalidate a valid displayed CLI graph; obsolete derived active pointers are not production query sources and matching-marker checks recover them.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-60 committed6fa641e after independent review/test,720warm samples and50-stage full verification. Planner and explorer completed read-only research against current source: installation currently clears history and auto-fits; Navigation::retain_known misses class/file/group focus; delayed layout fallback can re-enable auto-fit. Loaded descriptor intentionally survives folder-field editing. Reload is explicit in TASK-61; automatic analysis/adoption remains for opt-in TASK-62. Presentation-cache retention read/modify/save and multiple adoption actors are being checked before adding the reload consumer.

MCP feature first failed against the real CLI/MCP stdio boundary with method-not-found, then passed after independent wire/tool implementation. All36MCP tests passed in release (23unit/13processintegration inclrealcompiler/cancellation/comparison); boundarycheck passed89dependencies; all9affectedMCPsources CodeHealth10. Initial sandbox-only unit run had process-inventory denial and was repeated onhost; an initial debug ignored-suite command was corrected to required release. Independent review cleared MCP and passed4wire tests. Desktoptransport firstred failure-as-stale, retry-cached-capability and invalid-contract-match regressions were fixed; same-project history-loss regression redthen green. Adoption/cache firstconsumer now singlewriter with UI-owned session/marker, intentionally decoupling valid display from write-only profiling retention; realSQLite tests are being updated to assert the new boundary rather than background session mutation.

Final independent source review cleared all findings. Independent desktop validation passed20focusedtests, then reran finalactualcompiler/Rune/Metal journey5.77s withloaded-header invariants,terminalmissingartifact/manualRetryand7captures. Parentvisualinspection caught/fixedoffscreenfontatlasreuse andpending-folderheaderlabel; seven regenerateddark/lightcompact captures are legible, Reload/Retry reachable, fittedview readable. Same-primary-project realCLI/SQLite @sharedA->B remap preservesdigest/IDs/navcamera butupdateshydratedsource/cacheand rejectsoldbindingwithoutrequiringepochchange. Rootordinary240passed/9ignored,75projectionintegrationspassed,releaseClippy passed. MCP36release andindependent7focused+4boundary tests/actualoldCLIprobe passed. All46touchedRustfilespassscopedformatting. Fullworkspaceverification nowstarting; evidence at evidence/freshness/consumer-adoption/README.md and evidence/freshness/task61-desktop/README.md.

Full verification passedall72stages, includingcomplete releaseacceptance/benchmarks, packaging/signatures,installedCLI,independentMCP/agentjourneysandCodeScene. Pre-commit safeguardpassed46eligiblefileswithoutregressions. Sourceindependentreviewcleared; finalactualcompiler/Rune/Metaljourneyindependentlypassed5.77swithall7capturesinspected. Sonarinactive(no.sonar/config.json). Reportsat evidence/freshness/consumer-adoption/verification/. Localcommitoncurrentbranchfollows; no push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered typed asynchronous desktop freshness and explicit generation Reload, preserving valid selection/history/camera while rejecting obsolete epoch/root/generation jobs. Project changes reset; same-project linked-root remapping hydrates the new source addresses. Derived cache retention is single-writer and cannot own saved session identity. Added independent CLI-only MCP graph_freshness with optional capability negotiation and strict wire/envelope validation; older tools remain usable. Real compiler/Rune/Metal, SQLite, MCP/process tests and seven inspected captures pass. Full verification72stages and CodeScene46eligiblefiles passed. Evidence/actionlog: evidence/freshness/consumer-adoption/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
