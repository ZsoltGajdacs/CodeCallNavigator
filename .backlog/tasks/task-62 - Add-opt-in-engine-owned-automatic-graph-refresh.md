---
id: TASK-62
title: Add opt-in engine-owned automatic graph refresh
status: Done
assignee:
  - '@codex'
created_date: '2026-11-13T08:06:00Z'
updated_date: '2026-11-13T18:07:00Z'
labels: []
dependencies:
  - TASK-60
  - TASK-61
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once saved changes are visible, users need automatic coherent graph updates without repeated manual refresh. Keep scheduling in an engine coordinator and use existing one-shot analysis subprocesses initially so per-refresh cancellation stays separate from monitor shutdown. Full compiler-backed extraction remains the correctness baseline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Automatic refresh is explicitly opt-in, manual refresh remains available, and one active analysis plus one coalesced pending refresh bounds per-project work.
- [x] #2 Debounce, invalid-code recovery and lock contention do not loop on unchanged failures or canonical/cache/build-output writes; input changes before publication retain the prior artifact and schedule the latest pending state.
- [x] #3 Cancellation of one refresh, explicit user cancellation and monitor shutdown have separate outcomes and reap owned descendants without resetting the global cancellation flag.
- [x] #4 Concurrent desktop/agent/manual writers use the existing publication lock; queries retain generation binding and consumers never combine generations.
- [x] #5 Per-language publication budgets are derived from TASK-59 evidence and confirmed with at least 20 warm samples on each claimed supported workload; no unmeasured universal latency promise is made.
- [x] #6 Real compiler concurrency/cancellation/edit-burst journeys, full verification and CodeScene verify failure recovery and process cleanup.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve exact selected build identities before analysis consumes targets. Add optional portable analysis.selection v1 with path/kind/language while leaving legacy analysis.targets unchanged; core owns path resolution. Preserve future/opaque selection metadata in basic graph loading and normalize only the owned supported section. Unsupported or incomplete legacy selection must not be guessed from discovery defaults.
2. Add explicit CLI automatic-refresh capability and opt-in observe mode, keeping passive observations and one-shot queries compatible. Keep refresh activity separate from input validity and volatile state outside canonical graph artifacts.
3. Introduce an engine coordinator with debounce, one active one-shot serve-once child and one coalesced latest pending revision. Resolve each request from the latest executor-owned artifact and exact selection, reuse full analysis/publication rules, and classify busy/changed-input failures structurally rather than matching messages.
4. Retain existing process supervision and cleanup through a focused owned analysis-child interface: bounded/drained protocol output, cooperative per-refresh cancellation, descendant identities, finite escalation/reaping. Distinguish superseded refresh, explicit user cancel and monitor shutdown. Never reset the process-global cancellation flag; every analysis runs in a fresh process.
5. Use a separate per-project coordinator lease while children acquire the existing AnalysisLock. Register lease/output paths in canonical exclusions. Handle manual/agent competition with bounded busy-state reconciliation, suppress unchanged invalid-code retries and preserve the last successful artifact.
6. Extend desktop opt-in only for the loaded primary project, default off and cleared on project changes; editing the pending folder cannot retarget it. Keep scheduling/retry policy in CLI, retain manual refresh and use the tested bound reload/adoption path automatically only while authorized. Give auto monitor shutdown enough background grace to reap analysis children; passive lifecycle remains compatible.
7. Add real compiler/filesystem journeys for bursts, invalid new-file repairs, changed-before-publication, exact/empty/mixed targets, root remapping, competing writers/leases, cancellation/EOF/signals/blocked stdout and descendant cleanup. Use narrow pure scheduling/state tests alongside real integration coverage.
8. Build a reproducible publication experiment from existing edit/resource/comparison helpers: two independently analyzed clean body variants, alternate valid saves for at least20warm actual automatic publications on each claimed TypeScript/C#/Maven/Gradle workload, compare complete artifacts, retain raw stages/resources/limitations and derive per-route budgets from TASK-59 plus these results.
9. Independently review/test coherent checkpoints, inspect the opt-in GPU journey, run scoped formatting/CodeScene and required full verification, finalize evidence and commit TASK-62 on the current branch. Leave compiler reuse optimizations outside this delivery unless a separate measured task establishes safe gains.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-61 committed19b6b8a after72fullverificationstages, independentdesktop/MCP review/testing and7inspectedGPUcaptures. Planner/explorer confirmed exacttargethistory islost in legacy path-only provenance: same-directory Maven/Gradle selections collapse, including emptygraphs, and folder_project_builds/identity_roots cannot reconstruct them. New stable selection provenance is needed; legacy unsupported setup must request explicit manual refresh rather than guessing. Observe currently skips signals and desktop stops its child after200ms; auto mode must instead cooperatively reap independently grouped analysis descendants before escalation. Opt-in is scoped to loadedprimaryproject/session, not pendingfolder or a global carried preference. Core/engine/desktop refactorings will be extracted beside first consumers with behavior-preserving checks.

TASK-63, TASK-60 and TASK-61 are committed on the current branch after independent review and full verification. TASK-62 implementation is split by ownership: core/engine exact selection and refresh coordination; desktop project-bound opt-in and adoption; independent real-compiler/process acceptance; publication measurement and final integration. The measurement harness uses two independently analyzed clean body variants, carries accepted stream sequence boundaries, and compares complete artifact bytes for every automatic publication. Seven focused harness regressions pass; independent TypeScript reference parity and harness review are clear. Runtime automatic-refresh measurements and full verification remain pending.

Checkpoint evidence: core selection/lock/protocol tests 96 passed; negotiated one-shot failure and intervening-generation tests 2 passed; legacy clear-cache tests 4 passed. Independent review cleared portable exact selection, opaque future metadata, typed contention and the generation precondition under AnalysisLock. Desktop ordinary tests passed 249 outside the sandbox (10 ignored); source review identified and fixed frozen-manual-request, monitor handoff, optional-auto/passive compatibility and cleanup deadline issues. A shared core process-ownership primitive now centralizes high-precision PID/start identities for engine and desktop; focused independent lifecycle review is clear. The automatic coordinator, real compiler journeys, publication measurements and final full verification remain pending.

Repeated publication research found two issues beyond the initial acceptance matrix: generation-only observations could supersede an already-published child, and Java-owned class files changed the general observed revision during compilation, causing extra attempts. The generation race has a red-to-green regression. Java now records portable versioned refresh_inputs ownership from evaluated class directories, with per-input filtered directory baselines; public freshness remains unchanged. Six Maven/Gradle diagnostic publications match independent full artifacts with one analysis each. Independent review is checking dependency/alias overlap and filtered coverage guards before the final 84-publication campaign. Raw failed campaigns and output-change traces are retained under evidence/freshness/automatic-refresh/. No final latency targets or completion claim yet.

Final reviewed measurement passed all 84 publications (first plus 20 warm per TypeScript, C#, Maven, Gradle), each exact-byte equal to independent clean references with counters 1 through 21. Warm publication p95: 1.557s / 7.455s / 3.371s / 7.195s; stale detection p95: 263.3 / 261.1 / 261.7 / 262.9ms. Fourteen real compiler/filesystem/process acceptance cases pass, including 65536-byte undrained output shutdown; eight separate passive/automatic idle windows completed without analysis or artifact writes. Final source review is clear after output dependency, symlink binding and nullable activity guards. Core106, engine ordinary release, strict Clippy, real producer fixtures and native desktop journey pass. Source CodeScene evidence and raw campaigns are in evidence/freshness/automatic-refresh; .gradle is an unsupported scoring extension, covered by real Gradle fixtures. Full verification is still running before final acceptance checks and commit.

Final verification: all 92 full functional/performance/packaging/integration/CodeScene stages passed. The full command exited 1 solely because final cleanup received an inconclusive native lsof result; the unchanged protected cleanup passed on retry after an idle native inspection. Original results and successful retry are retained under evidence/freshness/automatic-refresh/verification/. No product changes followed the frozen measured executables. All 17 verification-quality parser regressions passed again. Final evidence: 84 actual automatic publications (first plus 20 warm per TypeScript/C#/Maven/Gradle route), complete-artifact parity against independent full analyses, 14/14 real compiler/process acceptance cases, eight no-edit idle windows, and the final release Metal desktop journey passed. Independent source and measurement reviews are clear; numeric supported-source CodeScene reviews score 10, declaration-only core lib is accepted by the narrow gate, and Gradle script scoring is unsupported and explicitly recorded. All four delivery tasks are complete on the current branch; compiler reuse remains deferred.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered explicitly opt-in CLI-owned automatic refresh and desktop adoption with exact selection provenance, bounded debounce/pending work, isolated analysis cancellation, owned descendant cleanup, shared atomic generation-bound publication, Java output-loop prevention and preserved manual/passive behavior. Measured publication p95 is 1.557s TypeScript, 7.455s C#, 3.371s Maven and 7.195s Gradle on documented local fixtures; route-specific budgets and limitations are retained. All 84 measured publications match independently analyzed references. All 92 full verification gates passed, with final protected cleanup succeeding separately on retry. Evidence and reproduction: evidence/freshness/automatic-refresh/README.md and evidence/freshness/task62-desktop/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
