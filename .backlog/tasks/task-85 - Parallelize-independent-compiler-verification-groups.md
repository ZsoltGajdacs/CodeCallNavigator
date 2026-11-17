---
id: TASK-85
title: Parallelize independent compiler verification groups
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T19:43:00Z'
updated_date: '2026-11-17T20:35:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measured Slice2 verification took 40m50s. Rust integration alone took14m45s, while the runner limits threads inside each test binary but executes binaries/groups sequentially. Slow late feedback makes iteration difficult. Improve scheduling without reducing selected coverage, loosening isolation or weakening cleanup.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Independent boundary-test groups execute concurrently within one bounded compiler budget
- [x] #2 Declared serial groups cannot overlap, and native-window and performance stages remain exclusive
- [x] #3 Selected test identities, environment groups, failure propagation, logs and process cleanup remain correct
- [x] #4 Tooling regression tests prove concurrency bounds, serialization and failure handling; subsequent full verification records timing comparison
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Use existing four-job boundary budget across Rust executable groups rather than four threads per sequential executable. Preserve unit behavior and all inventory selection. Group shared serial resources into exclusive work chains; execute independent chains through the existing standard-library executor. Keep stage/category scheduling unchanged so native/performance gates stay exclusive. Validate tooling in an isolated checkout through the deterministic selector, commit this independently, bring it into the engine checkout, then run complete final Slice2 verification and compare timings. No application test selection or timeout changes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created isolated branch codex/parallel-verification-20261117 at /private/tmp/ccnavigator-parallel-verification-20261117 so tooling verification excludes unfinished engine changes. Planner/explorer reviewing scheduling ownership; tester owns tests/inventory; implementer owns runner after observed regression red. Budget remains four total boundary tests, each concurrent group single-threaded; unit execution and category/stage exclusivity unchanged. Root engine changes are not copied into checkout. Full Slice2 baseline changes-od_g1ux1 completed66/66passed,2450.005recordedstage seconds; final CSharp boundary consolidation still requires rerun. Backlog initial create failed on repository-local lock permissions, then succeeded through authorized escalation; no task content edited manually.

Tester added five real subprocess-protocol scheduler regressions in isolated checkout, explicitly classified integration/tooling (not compiler correctness evidence). They cover global four-job overlap/bounds plus exact/env/ignored selection, cross-binary serial groups with independent overlap, native/performance exclusivity including mixed groups, pending-stop/active-drain failure behavior, and missing-coverage validation before launch. Production API rust_run unchanged. Selector-only red run next; implementer waiting for expected failure. Existing isolated checkout lacks root engine changes so tooling plan selects no application builds.

Observed behavioral red in isolated changes-5exoyjro: independent overlap and cross-binary serial-plus-independent rendezvous timed out; failure case expected exit7 but sequential barrier returned1; missing-selected-executable validation occurred after a test started. Native/performance exclusion passes existing serial runner as expected. Pythonintegration279cases,2existing skips; cleanup passed. Test sources CodeHealth10.0. Existing unit-harness/default and serial1 policy assertions retained; boundary invocation assertion changed to1 because four-job budget moves to executor. Production implementation now underway; native/performance invocations inside Rust groups also run exclusively, not just outer gates.

Production scheduling implemented in isolated suite_execution.py, CodeHealth10.0: preflight all selected artifact coverage, explicit RustGroup work, bounded four-chain executor with one boundary test thread, cross-binary serial chains, exclusive unit/native/performance batches, worker-side failure event, pending stop and active-drain before deterministic whole-group logs. Root reviewed diff; independent reviewer checking correctness. Strengthening existing probe for ordered complete logs before green selector. Resolved two red-run prerequisite skips without code changes: copied pinned Lombok artifact unchanged into isolated vendor cache (SHA256match), and green run will use just-verified real packaged CLI plus existing TypeScript modules for compiler helper journey. No application build in isolated scope.

First green attempt changes-e5pf8qa9 passed all five new scheduling probes and unit tooling, but entire run failed on two unrelated existing tooling boundaries before new probes: supplied package CLI was overridden by SuiteContext with unbuilt category-local engine path; quality timeout reported killpg PermissionError/process-group cleanup unavailable. Run was outside sandbox. Prior red run same cleanup probe passed. No scheduler/timeout/selection weakening. Rerun complete tooling plan with normal PYTHON-only environment: optional compiler journey is intentionally opt-in and will be reported notrun, not passed; candidateCLI ownership bug not expanded into this task. Pinned Lombok dependency is now present so that former skip must execute. Independent scheduler review found no actionable issues; safeguard passed3sourcefiles.

Verified checkpoint: isolated changes-58yku9rw passed15/15computed stages, including all five new process-scheduling probes, unit policy checks, surrounding integration/tooling checks, cleanup and CodeScene. IntegrationPython273ran with1optionalcompiler-journey skip (notpassed; tooling-onlyplan builds noCLI); pinnedLomboktest and previously flaky qualitytimeout both passed. No test selection/timeout changes. Independent review found no actionable issues. All3modifiedPython sources score10.0; safeguard passed isolated3sourcefiles and root16eligiblefiles. Transferred exact staged patch into root while keeping allengineSlice2edits unstaged. Commit separate tooling checkpoint now; AC4/full-application timing comparison remains pending finalSlice2selector.

First full-application measurement with committed runner8429963 and final CSharp guard consolidation: changes-i86p68hx integrationRust passed384.107s vs baseline changes-od_g1ux1 885.473s (about56.6%less walltime). This measures combined scheduling plus redundant-MSBuild removal, not isolated attribution to parallelism. Exact selected-identity comparison is being recorded; full acceptance/performance still running. No skipped test IDs, timeouts or verification scope changes used for speedup.

Completed matching-stage comparison in finalSlice2verification changes-i86p68hx: exactly664same integrationregistrations and1samegate as baseline; Rustintegration885.473to384.107s (~56.6%less), Rustacceptance287.195to234.938s (~18.2%less), CSharpflowCLI381.669to343.937s. Numbers reflect combined bounded scheduling and removal of redundant app MSBuild verification, not isolated scheduler attribution. Final app plan stopped at user-approved desktop/later-check deferral; no complete elapsed/full-green comparison claimed. Original interruption/cleanup failure evidence retained; independently retried ownedcleanup passederrors[] after transient nativePID exited. Tooling-only15stagevalidation plus all5schedulerprobes passed; one optional compiler helper notrun there (notcountedaspassed). Native/performance exclusivity remains in force; finalSlice6stillowns deferred checks. Separate toolingcommit8429963, no push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Parallelized independent Rust boundary groups under a global four-worker budget while preserving shared serial resources, exclusive unit/native/performance work, exact inventory coverage, environment flags and deterministic complete logs. Failures stop pending chains and drain active children. Five real process-scheduling regressions, isolated tooling checks, independent review and CodeHealth passed. Full-application matching integration stage fell from14m45sto6m24s with unchanged selected identities, jointly with CSharp guard consolidation. Remaining deferred native/later checks stay scheduled for finalSlice6.
<!-- SECTION:FINAL_SUMMARY:END -->
