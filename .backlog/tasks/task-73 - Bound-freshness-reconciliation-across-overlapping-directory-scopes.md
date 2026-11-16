---
id: TASK-73
title: Bound freshness reconciliation across overlapping directory scopes
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T18:26:00Z'
updated_date: '2026-11-16T19:20:00Z'
labels: []
dependencies: []
type: bug
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Live investigation of Recurring-billing found the passive observer saturating a CPU core inside Inputs::finish with 1677 recorded directory scopes. Each scope recursively traverses overlapping descendants and each path linearly scans all scopes. Startup unmapped_input375 is a separate recorded coverage limitation; output ownership is now complete.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Overlapping recorded directory scopes share traversal work while distinct exclusion policies and explicitly reopened child scopes retain their semantics.
- [x] #2 Content revisions, input status, symlink uncertainty and cancellation remain correct; a deterministic work-count regression catches redundant traversal.
- [x] #3 The complete verify-changes.sh computed plan and required changed-source Code Health checks are run and their actual outcomes recorded.
- [x] #4 Java build progress does not falsely promise that custom project build steps cannot run tests.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Capture live observer stacks and saved-artifact coverage, and trace the user manual-refresh retry.
2. Group traversal scopes by exact absolute exclusions and reuse existing traversal deduplication; index exact scope paths.
3. Add pure planning and real-filesystem work-count/content regressions; execute tests only through verify-changes.sh.
4. Review and run the complete deterministic selector, required Code Health and owned cleanup.
5. Report the actual manual-refresh handoff, persistent unmapped-input limitation and rebuilt package. Use the selector regression counters and benchmarks for post-fix validation; launch no additional off-selector verification against the live app.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer confirmed repeated traversal and linear scope membership in live::Inputs::finish. Safe plan groups scopes by exact absolute excluded-output vectors, retains every explicit root and distinct policy, and indexes exact scope paths. Current artifact has 1666 scopes with empty exclusions plus eleven distinct nonempty policies. Startup unmapped_input375 is a separate portable-manifest limitation; the scheduler can use complete filtered trigger coverage despite public Unverifiable status. The exact unmapped paths are intentionally absent from the artifact, so their full composition is not claimed. Added pure traversal-planning and real-filesystem regressions; complete selector red run underway. Selector selected shared-core consumer correctness and core/engine performance; no raw or hand-selected test commands are being used.

Live manual retry traced: engine62883 invoked Maven, whose UI module antrun compile phase ran npm ci, npm test (vitest run) and frontend build despite standard skipTests flags. The analysis child exited and a new graph was published; desktop catalog and presentation cache adopted generation9ec559bb6d2b4b083e2657f55e0a77f658786bc5b87ce4955a490277d82123b3. Thus this retry completed end-to-end; startup unmapped375 is not evidence of a failed manual refresh. The POM has an existing skipUi switch usable via MAVEN_ARGS for an individual app launch; no project-specific flag is being hardcoded into the analyzer. Correcting the misleading Java progress claim that no tests run, since custom build plugins can run tests. Pure traversal regression failed under the complete selector red run; cleanup passed. Final grouped traversal/index implementation awaits review and green selection.

Independent review found no actionable findings: batching preserves lexical roots, exact absolute exclusion policies, canonical revision values, explicit generated/symlink roots and cancellation. Java progress now truthfully says Building Java source sets. All three changed Rust source files received Code Health10.0. Starting the complete 55-stage focused verify-changes.sh green plan with required consumer correctness and core/engine performance selections. Live user app/release binaries remain untouched; new policy forbids raw tests, manual categories and custom continuation plans.

First green selector stopped at integration-test-rust: changing Java progress from Compiling to Building prevented an existing real-build cancellation/mutation test checkpoint (message.contains Compiling) from firing. This is a regression from the progress wording edit, not the traversal optimization. Preserve the existing Compiling prefix and remove only the inaccurate no-tests claim; do not weaken the cancellation assertion. Prepared cleanup passed; later acceptance/performance/quality stages are not run. Rerun the complete computed selector to fresh evidence after the minimal correction.

Selector green-r2 now passed the formerly failing Maven cancellation/mutation checkpoint unchanged (all nine Java build cases), actual desktop/CLI reload and opt-in automatic refresh integrations, and all Rust/Python integration artifacts. Exact new filesystem passes are in target/verification/task73-green-r2/22-integration-test-rust.log lines913 and915: bounded overlapping-scope traversal/content and explicit generated/symlink roots. The work-counter regression observes each of17 nested directories once and hashes the unrecorded source once. Selected Node integration and later acceptance/performance/quality remain pending; full computed-plan acceptance is not yet checked.

Final green-r2: all56 computed stages passed. The new grouping, bounded filesystem traversal/content and explicit generated/symlink-root regressions passed; native acceptance, automatic-refresh acceptance, consumer correctness and selected core/engine benchmarks passed. Final source Code Health is10.0 for live.rs, observation_tests.rs and java.rs, with safeguard passed for3 eligible files. Timing: compilation360.256s, setup17.825s, tests/gates1270.899s, cleanup44.012s, other/static+packaging+quality120.778s; recorded total1813.770s. All five owned cleanup reports passed without errors and reclaimed9,714,779,333 bytes; required package rollback retention is181,463,302 bytes. Evidence: target/verification/task73-green-r2/summary.json and56-codescene.log. No extra live-graph query or off-selector verification was run. The running user app/release binaries were left untouched; restart with dist/CodeCallNavigator.app to use the rebuilt implementation. No commit or push performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Investigation confirmed the original ownership blocker is resolved: the real saved graph has complete compiler-output ownership. The traced manual refresh successfully published and the desktop adopted its new generation; Maven also ran frontend npm installation/tests/build via custom compile-phase Ant tasks. The startup unmapped_input count375 is a separate saved-manifest coverage limitation and remains truthful. Fixed pathological freshness work by batching identical absolute exclusion policies and indexing exact scope membership, preserving exclusions, explicit generated/symlink roots, canonical revisions and uncertainty. Removed the inaccurate no-tests progress claim while preserving the Compiling checkpoint. Added3 regressions; the complete current verify-changes.sh plan passed56/56 stages with Code Health10.0 and successful cleanup. The rebuilt package is available in dist; current live-app performance after restart was not separately measured outside the selector.
<!-- SECTION:FINAL_SUMMARY:END -->
