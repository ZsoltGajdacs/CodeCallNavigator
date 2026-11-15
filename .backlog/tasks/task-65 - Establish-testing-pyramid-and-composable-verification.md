---
id: TASK-65
title: Establish testing pyramid and composable verification
status: Done
assignee:
  - '@codex'
created_date: '2026-11-15T09:32:00Z'
updated_date: '2026-11-15T13:35:00Z'
labels: []
dependencies: []
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement approved testing strategy on current branch preserving coverage and work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Module guidance defines testing pyramid real code and narrow fault probes
- [x] #2 Exact inventory covers tests and standalone gates with category and requirements
- [x] #3 Light and full compose category and component scripts with safe automatic cleanup
- [x] #4 Coverage-preserving migration has timing evidence and required verification
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Baseline current branch; classify registered tests; compose category scripts; safe cleanup after each category; migrate redundant execution; independent review and full verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Current branch feature/task-22-function-flowcharts was clean at start and remains unchanged. Added module guidance, explicit inventory and composed category/component scripts. Independent review replaced nested runners with a single process owner. Unit category passed: 57.521s execution, 48.965s compilation, 2.393s cleanup; removed 4,521,120,258 owned build bytes. Corrected initial inventory: root Rust integration targets, duplicate binary registrations and inherited Python cases were missing from earlier source counts. Current 1345 executable registrations and 11 standalone harnesses are inventoried. Full prerequisites pending private Recurring Billing checkout path; asked user and continue light/integration checks. No source tests removed; migrated duplicate debug/release execution and isolated TypeScript 2 pure unit cases from 110 integration cases.

Full run full-80xaq5kd passed all functional unit/integration Rust, Python, Node and C# flow gates; integration Rust took 1508.471s. Cleanup correctly failed closed on .NET build servers holding the category scratch directory. Verified exact PIDs/start times/private shared-memory bindings, stopped only those two owned servers, and added isolated verification SDK server-disable policy. Found both private Java fixtures unnecessarily run the external checkout frontend tests; use its existing skipUi property only in those two child test environments, preserving all Java assertions and inherited Maven flags. Moved all 11 core flow cases (45 assertion macros unchanged) into core crate with exact inventory identity mapping. Added selected-requirement preflight and language-scoped bootstrap; full default unchanged. Restarted full verification with provided private checkout and profiling snapshot; no branches/worktrees created.

Full-jdvmsl1v passed unit, all integration (Rust 880.696s), packaging/signature, independent MCP install and Rust acceptance (330.518s); category cleanup passed after .NET server policy fix. Both private Java cases passed with existing skipUi property and unchanged assertions. Automatic-refresh standalone gate passed 13/14 cases but undrained-output setup relied on invalid macOS pipe byte threshold. Replaced only fault-probe transport with real bounded Unix socket, verified kernel backpressure after real initial CLI frame; focused packaged-CLI case now passes with original shutdown/descendant assertions. Added one real transport integration registration (inventory now 1352 plus 11 harnesses). Restarted complete full-c0mlqq73; awaiting native/performance gates. No production API/wire changes or branch/worktree changes.

Review fixed standalone environment defaults and legacy freshness CLI alias, between-stage SIGTERM cleanup, missing standalone harness discovery, and repeated reclamation overwriting cleanup byte receipts. Real held-file and active-child/inter-stage SIGTERM regressions pass; modified sources CodeScene 10.0. Applied 13 evidence-backed unit-to-integration classifications without body changes. Moved another 72 core registrations (264 assertions) into core, retaining the real C# entry case in root. Source-audited explicit serialization groups replace blanket non-unit Rust serialization; unaffected artifacts stay sequential and Python stays serial for mutable globals. Current full original unit/integration phases passed; Python stage reported one skipped legacy freshness journey due unset alias, now fixed and requires affected rerun. Post-review unit/integration category runs will validate new placements and parallel execution after current full releases its lock.

Acceptance category acceptance-j2v2_htb passed all21 stages: 14 automatic-refresh scenarios, native C# light/dark, native comparison light/dark, installed CLI and MCP journeys. Inspected native screenshots against existing evidence; production UI unchanged. Cleanup reclaimed estimated 1,534,457,549 logical bytes and retained required rollbacks. Core-only unit-sz95zxpi passed109 registrations (8.087s execution,9.597s compilation,4.117s cleanup;508,061,663 bytes removed); compiledtesttargets onlycore. Performance-gpu747i3 passed all10 stages with isolated profilingcopy (19.978s execution,110.408s compilation,7.573s cleanup), real Metal Apple M4 Pro adapter. Found two older C# baseline gates writing fixed /private/tmp fixtures; moved newmaterialization into ownedwork with expected physical-prefix-only rebasing, preserving actual compiler output and canonical/historical controls. Added5 regressions (inventory1358+11). Focused engineintegration and packaged-ingestion reruns active. Nativehelpers also bypassTMPDIR and require owned interrupted cleanup while preserving failedfixture evidence; implementing minimal shared ownership/retention support before final complete full composite. Earlier partial full failures remain explicitly failed; no final full success claimed.

Owned C# ingestion recheck passed against packaged CLI with all recorded projections preserved. Focused engine integration-68ir_nk5 passed all9 stages (233 registrations;559.482s Rust,30.379s Node,4.539s raw C# gate;95.960s compilation,4.743s cleanup;1,282,050,431 logical bytes reclaimed). New NativeFixture now binds both native gates to category TMPDIR, defers safe deletion, retains interrupted/failed diagnostics only after idle proof, and handles stale interrupted marker writes atomically; real mini cleanup CLI/held-file tests passed and CodeScene10. Final full-cl476dyr passed compilation/Clippy but cleanup refused an inconclusive native lsof result, preserving458,275,978 bytes. Fresh nativeinspection then proved idle; same guarded cleanup removed them successfully. No safety check relaxed. Corrected retained-byte summary to count blocked/unavailable leftovers and expose unknown-size paths; added bounded lsof exit/output diagnostics. Focused regressions and CodeScene10 passed. New complete full attempt active (session52989); source otherwise frozen. Cargo auto-GC is disabled for verification to preserve downloaded dependency caches per approved policy.

Final full-enbwvtxc PASSED all112 stages, including native light/dark journeys, Metal/CPU/storage benchmarks, package/signatures, independent CLI/MCP, CodeScene51 changedsource reviews and everycategory cleanup. Exactselection union proves1358 registrations +11 harnesses eachselectedonce withoutomissions/duplicates; Python integration258tests no skips. Fullunit17.795s execution vs61.037s compilation,0.958s setup,4.962s cleanup. Four categories reclaimed8,941,891,272 estimatedlogicalbytes; allownedbuild/temp paths absent; rollback/evidence retained. Light-9t8sexpr PASSED all83stages (407unit+635ordinaryintegration) on final runner including exact ignored/ordinary groups. Durable evidence: evidence/testing-strategy/validation.json, distribution.md, migration.md, toolchains.json and preservation mappings. No production publicAPI/wire changes; no otherbranch/worktree. SonarMCP inactive because configabsent. Independentreview found no unresolvedblockers. Staged precommitCodeHealth safeguard remains before localcommit.

Final staged pre_commit_code_health_safeguard PASSED:51checked/eligible sourcefiles, no issues. Evidence saved as evidence/testing-strategy/quality-precommit.json. Allacceptancecriteria verified; taskcomplete. Changes will be committed locally on feature/task-22-function-flowcharts; no remote push/integration requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Established module testing policy, exact inventory and category/component entry points with light/full composites. Moved83core cases while preserving309 assertions, removed duplicate profile execution, scoped private Java builds, and implemented ownership-safe cleanup and retained native diagnostics. Full112stages and light83stages passed; all1358 registrations and11harnesses covered. Unit execution17.8s with compilation reported separately; categorycleanup reclaimed8.94GB estimatedlogicaldata. Evidence under evidence/testing-strategy. Work stays on original branch; no push.
<!-- SECTION:FINAL_SUMMARY:END -->
