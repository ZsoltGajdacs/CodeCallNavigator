---
id: TASK-82
title: Expose shared engine continuous analysis sessions
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-18T12:33:00Z'
labels: []
dependencies:
  - TASK-81
ordinal: 88000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Initial analysis and automatic refresh currently use separate lifecycle paths. Slice 4; engine owns recovery for all clients.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Public CLI and versioned machine protocol support initial continuous sessions without saved graph
- [x] #2 External edits coalesce with capped backoff and no retry limit
- [x] #3 No duplicate lifecycle invocation on unchanged external revision; cancellation reaps children
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Share one publication path between one-shot and continuous analysis. Keep one-shot graph ownership and preserve the project lock, expected generation check, final input guards, digest, and atomic publication.
2. Harden existing owned process supervision before retaining preparation. Prove the spawned group and observed descendants stopped on success, failure and cancellation; treat unknown cleanup as terminal, preserve evidence, and bound obsolete preparation directories. Reuse scoped native server settings rather than shutting down shared servers.
3. Retain a fixed selection and one PreparedAnalysis per engine session. Observe actual consumed inputs locally before the first graph exists and adopt the accepted candidate baseline after publication. Distinguish compiler inputs from owned extraction outputs, including real classpath consumers and no-source operations. Reuse valid stage results and native incrementality. Preserve known Maven timestamp limitations as actionable exact-path errors when changed bytes cannot be shown to have been compiled.
4. Use one engine retry policy for initial and saved analysis, with 300 ms debounce, capped backoff, coalesced pending work, and no retry cap for genuine edits. Retain only the last failed consumed/settled input pair to stop identical producer loops without hiding external edits or undo. Keep per-attempt cancellation separate from terminal disconnect and signal shutdown.
5. Expose continuous mode through CLI and the versioned machine contract, with the same publication, cancellation and typed outcomes. Desktop and MCP adoption belongs to Slice 6. Verify real fresh sessions, unchanged and timestamp-only inputs, native invocation counts for selected changes, output-only extraction, repeated failure and recovery, preserved artifact bytes, and complete child cleanup.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Started after Slice 3 commit 25c1687. The shared locked publication operation takes a native graph producer and attempt cancellation/progress. One-shot retains the move-only graph path, normalized selection, validation and atomic publication. Independent review passed. Process supervision changes are in progress with typed cleanup uncertainty, bounded obsolete job directories and scoped .NET server settings matching the existing verification environment. Runtime verification remains pending for this slice.

Foundation focused run changes-ak196y_4 completed 31 stages and stopped on one TypeScript runtime failure. The worker published between the output-file check and process observation, so exit 0 could be mistaken for missing output. Recheck published output after a successful reaped exit. All category cleanup passed; preserve the prior failed report. Subsequent session/transport code is isolated from this verification checkout.

Foundation changes-nw7ev8dd fixed the output/exit race and passed Rust, Node, Python, C# flow, package and Rust acceptance. Three real refresh journeys then exposed transient macOS proc_pidinfo EPERM while inspecting a rapidly changing descendant. 47 passed, 1 gate failed, 9 not run; all focused category cleanup reports passed. The retained-session implementation was developed in a separate worktree during this run. It now shares retry/publication across fresh watch and saved observe, and adds real fresh failure/repair and Java output-only/native invocation assertions. Verify together with the strict bounded ownership re-observation fix before committing Slice 4.

Combined pass includes reviewed transient birth-identity handling. Retry only indeterminate native identity lookups for up to 100 ms against fresh strict process inventories. A complete process table may prove a candidate absent. Live unknown ownership, malformed rows, group continuity errors and genuine signal failures still fail closed. Both reporter and reviewer confirmed the previous three acceptance failures had the same native EPERM cause.

Combined changes-qgz42e0y stopped at cargo-check because moving the Maven timestamp guard accidentally carried the following test helper cfg attribute onto the production type. Reattached the attribute to the helper. Static/bootstrap and checks cleanup passed; no runtime tests reached.

Combined changes-iknlvl6v passed 74 stages, intentionally interrupted 1 before desktop, and left 14 later stages not run. Unit/integration, package/signature, independent MCP install, Rust acceptance and all 15 real refresh cases passed. Earlier burst, lock and concurrent-edit EPERM failures are resolved. Fresh watch recovered from initial invalid TypeScript and a later added invalid input, kept the same engine process and matched three independent clean artifact generations byte for byte; stable failures and timestamp-only rewrites stayed quiet. Java native invocation counters passed for changed selection, class-output-only re-extraction, missing classes and restored source timestamps. All cleanup passed. Applied reviewed method extractions for the mandatory Code Health safeguard after this run; final candidate verification pending.

Final exact candidate changes-6jx6ktlw passed 74 stages, including all 15 real auto-refresh cases, native Java invocation and class output checks, package signing, and independent MCP install. The pre-desktop guard interrupted C# cleanup acceptance and deferred the later physical window and dependent checks to TASK-84 as agreed. All four category cleanup reports recorded zero errors. CodeScene pre-commit passed 36 eligible files independently.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The engine owns initial and saved continuous analysis, coalescing, retry, cancellation, and atomic graph publication. Sessions reuse compiler preparation by selected input and re-extract class output changes without rerunning Java builds. The fresh fixture recovered in one process and matched independent clean graphs. The deferred native and later gates are recorded in TASK-84.
<!-- SECTION:FINAL_SUMMARY:END -->
