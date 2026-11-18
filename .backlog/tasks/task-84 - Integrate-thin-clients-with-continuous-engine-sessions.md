---
id: TASK-84
title: Integrate thin clients with continuous engine sessions
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-18T15:31:00Z'
labels: []
dependencies:
  - TASK-83
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
MCP and desktop must consume engine recovery rather than implement separate policy. Slice 6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 MCP CLI and desktop expose equivalent engine outcomes and codes
- [x] #2 Desktop opt-in works before first publication and cancellation stops sessions
- [x] #3 All clients use the new engine contract directly; stale versions fail explicitly and packaging/native acceptance pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reuse the engine observer and fixed-selection analysis session before the first graph. Add a strict advertised initial-selection request and report effective root mappings on publication. Keep freshness absent until a real artifact exists.
2. Bind desktop opt-in to discovered selected builds and stop the owned monitor on opt-out or selection change. Adopt the exact published generation with its effective roots and preserve existing loaded-graph behavior.
3. Require matching continuous engine capability and forward typed failure codes through desktop and independently installed MCP. Keep MCP refresh bounded and ordinary graph queries read-only.
4. Verify graphless repair, inferred linked roots, reselection, cancellation and prior artifact preservation. Complete the combined deterministic selector and resolve the deferred native, installed-client and performance gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The final checkpoint must resolve the accepted locked-desktop deferrals from Slices 1–2 and TASK-86. In the latest matched run, changes-wmwd6yi3, desktop stage 61 was interrupted during fixture preparation before GUI launch or WindowLease. Stage 62 (real CLI ingestion with parallel cases), flow comparison, installed CLI, agent CLI/MCP journeys, flow expansion, performance/query/storage benchmarks and the late CodeScene stage did not run. Earlier common checks, Rust acceptance, automatic refresh, C# cleanup and category cleanup passed. Source Code Health and commit safeguards passed independently. Run these deferred checks with an unlocked desktop at final verification; scheduling protocol probes do not replace the real compiler journey.

Slice 3 replacement run changes-q97ej3qj passed 58 stages through automatic-refresh acceptance. The pre-window guard interrupted C# cleanup acceptance before native-window startup. C# cleanup acceptance and all later native windows, CLI ingestion, installed CLI, agent/MCP journeys, performance and late CodeScene checks remain pending for the final checkpoint. Category/global cleanup itself passed. User reconfirmed that the desktop will be locked and that these checks can wait until the end.

Slice 4 final exact run changes-6jx6ktlw passed 74 stages through all 15 automatic-refresh cases. The guard interrupted C# cleanup acceptance prior to desktop startup. C# cleanup acceptance, native windows, subsequent CLI and MCP journeys, performance and late CodeScene remain part of the final checkpoint. All category cleanup reports recorded zero errors.

Slice 5 exact TypeScript 5.9.3 and 6.0.3 matrix passed 48 stages per selector through all 15 refresh cases. At both agreed guard cutoffs C# cleanup acceptance and eight later selected stages were deferred. All category cleanup reports recorded zero errors. Run the complete combined selected plan at the final checkpoint.

Started final integration after verified Slice 5 commit 06892a1. The selected desktop observer shares the engine session and retains publication root mappings; MCP explicit refresh requests and forwards native typed outcomes.

Full final checkpoint changes-7mku9bvh passed 146 stages, including all previously deferred native windows, installed CLI, agent/MCP, performance and cleanup. The late CodeScene safeguard alone failed on a mean-complexity threshold in desktop capability validation. Grouped independent version and command checks into typed tuple comparisons with unchanged comparisons. Final exact selector rerun pending.

Exact capability rerun changes-yif7btkk passed all integration tests and was cleanly interrupted during acceptance compilation when an independent run of the combined CodeScene stage uncovered a second finding in the already-committed fingerprint policy. Consolidated the shared hidden-directory and symlink exclusion predicate without changing membership or hashes. Code Health 10.0 on the reviewed policy. User authorized the locked-desktop exception for the repeat native run; the earlier full run passed all physical desktop, installed-client, performance and cleanup stages.

Exact final run changes-sr0ca59i: 129 passed, 1 interrupted at the agreed pre-desktop cutoff, 17 not run. All 15 refresh journeys, headless first publication with inferred roots, MCP protocol checks, packaging and cleanup passed. Earlier full run changes-7mku9bvh passed all native windows, installed-client and performance checks. The later behavior-preserving quality revisions passed the complete 100-file CodeScene review and safeguard independently. User accepted the locked-desktop exception and directed committing without further verification.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Desktop opt-in starts an engine-owned session before the first graph and adopts published generations with their resolved roots. MCP explicit refresh preserves typed engine failures. The final runtime prefix, independent CodeScene review, earlier native and performance results, and accepted repeat-run exception close the final implementation slice.
<!-- SECTION:FINAL_SUMMARY:END -->
