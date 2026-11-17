---
id: TASK-80
title: Separate compiler preparation from graph extraction
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-17T20:35:00Z'
labels: []
dependencies:
  - TASK-79
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whole-attempt orchestration unnecessarily couples extraction retries to build lifecycle execution. Slice 2.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Validated compiler results can be reused for extraction without rebuilding
- [x] #2 Independent unchanged selected builds are reused using existing fingerprints
- [x] #3 Native build caches and atomic publication are preserved
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Introduce an engine-owned reusable analysis session with validated prepared results and per-target raw graph reuse; retain one-shot analyze as a wrapper. Complete Java compilation before TypeScript preparation so build scripts cannot invalidate programs prepared earlier in the same attempt. Preserve pre/post extraction and publication checks, validate all retained targets after builds, and keep locking at the entrypoint. Use existing fingerprints/native build reuse rather than custom dependency inference. Prove reuse and invalidation with real compiler fixtures; no compatibility branches. Native-window and subsequent blocked gates explicitly deferred by user to final Slice6.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented engine PreparedAnalysis fixed-selection/runtime API, native preparation before frontend capture, per-selected raw graph reuse, independent aggregate composition and consuming one-shot path. Java metadata overlap checks precede all compilation; compiled Java guards bracket extraction. CSharp retains CallAtlas output/ownership/import observations and re-evaluates import membership even for cached graphs. TS retains fingerprinted raw output for normalization retries. Only typed input changes invalidate caches; operational errors propagate; workers drop before obsolete-successful-directory cleanup. Provenance uses captured kind directly. Existing real Maven integration extended with actual invocation counters, cancellation/retry, independent build updates and bytecode invalidation. No mocks/rawtests; new-API preimplementation behavioral red unavailable, existing tests retained and actualboundary coverage used. Long-lived shared-intermediate cache pruning remains necessary in Slice4 before continuous hosting. Review and deterministic verification next.

First selector changes-69pfkc3_ stopped at cargo-check: moved C# helper cancellation import, C# test AtomicBool import and session job.path borrow. Corrected all three without rawcommands. Reviewer identified evaluated import membership inequality must be typed invalidation or cached preparation remains stuck; core InputsChanged constructor now supports explicit inventory changes, C# capture/verify and TS resume metadata inequality use it, Java module-membership changes likewise typed. Operational errors and cancellation remain distinct. Existing real-MSBuild marker tests assert typed path/reason. Captured-kind provenance test uses explicit in-memory snapshot. Re-run computed verification next.

Final read-only review caught extraction-time semantic InputsChanged that might escape saved path guards. SelectedJob now immediately clears that owning preparation/raw cache on noncancelled typed invalidation while retaining failed logs; operational failures and cancellation remain reusable. Code Health 10.0. Selector changes-pw7fyqns began before this small final edit, so its compiled Slice2 evidence excludes that branch; final computed-plan verification remains necessary.

Selector changes-pw7fyqns passed static/Clippy/unit checks but integration stopped at the new real Maven selective-rebuild assertion: revisedLeaf absent after a successful native rebuild (8/9 java_builds cases passed). Root cause: JVM ArtifactInventory is keyed only by artifact path, safe within the former immutable one-shot job but stale when a shared session spans new compiler outputs. Fix at owning session cache lifetime, invalidating only shared derived artifact inventories when preparation invalidates; preserve native outputs and unchanged raw graph parts. This is an observed behavioral red for retained-session cache lifetime.

Minimal cache correction avoids new dirty-state tracking: each extraction pass checks cancellation and removes only its owned shared artifact-inventories before helpers run, retaining within-pass sharing, native outputs and independent raw graph caches. Missing cache accepted; cleanup failures propagated. Also includes extraction-time typed invalidation fix. Code Health10.0. Complete selector rerun started for final stable bytes.

Stable rerun changes-y2idgx9j: static compilation/Clippy/unit checks passed, and maven_standalone_changes_and_failed_refresh_preserve_cached_graph now passed (including revisedLeaf, actual native invocation counts, independent unchanged target reuse, bytecode rejection and preserved prior publication). Remaining integration/acceptance checks still running. User reconfirmed commit Slice2 after verification, continue Slice3, and defer native-window checks to final checkpoint.

Stable rerun changes-y2idgx9j passed Maven reuse regression and both real Recurring generated-declaration journeys, but stopped late in integration on pre-existing zero_deadline_never_starts_process_and_cancellation_takes_precedence: marker unexpectedly existed. Diagnosed ProcessLogs::check_wait using elapsed > timeout; coarse Instant resolution may produce exactly zero before spawn. Correct deadline comparison is >=, preserving cancellation precedence. Existing real-process red test retained; minimal one-character boundary fix included so required verification can complete. Cleanup passed. Native acceptance not reached.

User requested runtime bottleneck analysis. Current stable run recorded integrationRust885.473s, acceptanceRust287.195s, unit execution18.223s; compilation/Clippy about270s plus packaging112s. Source inspection found avoidable Slice2 overhead: seven fresh CSharp import revalidations after initial metadata versus two in prior one-shot flow, because adapter and session both own stage checks. Plan reviewed before edits: keep cheap fingerprint guards at each boundary, centralize fresh evaluated-membership checks at session preparation entry/final preparation/final extraction, with no new cache or weakened final consistency. Current full run remains baseline; final changed bytes need selected verification again. Native desktop gate passed this run; final checkpoint still required.

Full selector changes-od_g1ux1 completed66/66passed with cleanup/performance/CodeHealth gates passed, including previously problematic nativewindow gate. It verifies pre-consolidation acceptance binaries, not final MSBuild check consolidation. Following user parallelization/iteration feedback, separate TASK85 adds bounded runner concurrency in isolated checkout before final Slice2rerun. Engine implementation remains in root, stablepending finalselector; nativewindow still recheckedfinalSlice6. Required cached CSharp semantic-import reuse coverage must be included when continuous client begins retaining CSharp preparation in Slice4.

Final stable-byte verification changes-i86p68hx: all static/compiler/Clippy/unit/integration/Rust-acceptance/automatic-refresh/CSharp-cleanup/CSharp-desktop/CSharp-flow-ingestion checks passed. Rust integration384.107s versus baseline885.473s with exactly664same integrationregistrations and1samegate; Rustacceptance234.938s versus287.195s. Source CSharp initial full semantic checks consolidated7to2 while per-stage fingerprint guards remain. At user-approved deferral boundary, sentSIGINT only to proven runownerPID60090 after CSharpflow CLI passed; remaining desktop/later/performance checks remain pendingfinalSlice6. Original runner records53passed,1cleanup-unavailable interrupted nativegate,1failedcleanup,11notrun; these records are preserved, not rewritten as allgreen. NativePID21680 had exited; safety-checked operational cleanup retry passederrors[] and reclaimed1.69GB, retaining native diagnostic fixture/report perpolicy. Initial cleanup failure report preserved alongside recovery. Rootprecommit safeguard passed13eligible sourcefiles; allchangedsourceCodeHealth10.0; Sonar inactive without projectconfig. No rawtests, no push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Separated fixed-selection engine compiler preparation from extraction, retaining validated CSharp maps, Java compiled inputs and TypeScript facts plus independent raw graph parts. Unchanged selected builds are reused; typed input changes invalidate the correct retained preparation, preserving native incremental builds and prior publication. Real Maven invocation counters verify extraction/cancellation retries avoid builds and a source edit rebuilds only the changed selection. Fixed stale path-keyed helper cache lifetime and the existing zero-deadline boundary defect. Non-deferred deterministic checks passed on final bytes, CodeHealth passed, and cleanup recovery passed. Remaining native/later checks explicitly deferred by user to finalSlice6; no whole-run pass claimed.
<!-- SECTION:FINAL_SUMMARY:END -->
