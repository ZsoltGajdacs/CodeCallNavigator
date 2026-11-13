---
id: TASK-59
title: Explore near-real-time graph freshness during code edits
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T13:48:00Z'
updated_date: '2026-11-13T08:56:00Z'
labels:
  - exploration
  - freshness
  - performance
dependencies: []
references:
  - TASK-4
  - TASK-30.12
  - TASK-39
  - DRAFT-1
  - DRAFT-2
documentation:
  - docs/standalone-engine.md
  - docs/graph-contract.md
  - docs/graph-freshness.md
  - evidence/freshness/README.md
type: spike
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
While an agent or developer edits a codebase, both agent queries and the desktop graph viewer should reflect the current code with minimal delay. Explore fast change detection and graph refresh so consumers can tell which source state their graph represents and avoid silently relying on stale context.

Existing manual refresh, portable freshness verification and inference-decision caching provide starting points. Inference cache reuse does not by itself establish incremental compiler analysis or continuous refresh. This draft seeks a measured feasibility assessment and recommended scope, including what near-real-time can mean for supported languages and project sizes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The report defines measurable freshness and responsiveness targets for representative project sizes and supported languages, distinguishing saved filesystem changes from unsaved editor buffers.
- [x] #2 Change-detection and refresh alternatives are compared with existing manual refresh, input verification and inference caching; compiler/build costs and safe incremental invalidation boundaries are identified.
- [x] #3 Representative edits, rapid edit bursts, file additions/deletions/renames and build/configuration changes are assessed for detection-to-publication latency, resource use and correctness against full analysis, with unavailable measurements identified.
- [x] #4 The report describes coherent graph generations and freshness states shared by desktop and agent consumers, including concurrent queries/edits, cancellation, invalid intermediate code and preservation of the last successful graph.
- [x] #5 A recommendation records feasible latency expectations, language-specific limitations, open decisions and implementation follow-up tasks without promising freshness beyond measured capability.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preflight and build a matching release CLI, discover toolchains and establish saved-edit query/refresh behavior.
2. Generate isolated two-source compiler projects for four routes, reuse the fixture Gradle wrapper, and copy the available larger Java corpus without local secrets or outputs.
3. Separate project mutations, CLI sessions, experiment collection and bounded process/resource measurement; collect one first repetition plus 20 warm samples per scenario.
4. Validate exact clean references and observable graph changes; retain C# provenance failures, missing root mappings, event-only observations and unmeasured production journeys explicitly.
5. Publish evidence/freshness/README.md and docs/graph-freshness.md; create TASK-60 through TASK-63 with dependencies and required refactoring boundaries.
6. Independently review and test, pass CodeScene and all 32 full-verification stages, reclaim idle build artifacts, and finalize the spike without shipping production monitoring.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Promoted DRAFT-3 preserving all five acceptance criteria. Approved scope is the feasibility spike and follow-up delivery tasks, saved files only; production monitoring and automatic refresh remain follow-up work. Existing packaged release source differs from current HEAD; provenance will be recorded explicitly.

Implemented measurement-only public-CLI harness and native fs.watch probe. Native first-event p95 approximately 41 ms across seven 20-repeat tiny-directory scenarios; this is not verified freshness or large-tree coverage. Four-language matrix started with isolated synthetic projects and shared warm dependency caches. Full verification target/verification/freshness-20261113 stopped at prerequisites: private billing checkout not configured; later stages not run. Follow-ups TASK-60 -> TASK-61 -> TASK-62 record typed observation, consumer adoption and opt-in refresh, including required refactorings.

Review corrected three measurement defects: Java target paths identify directories; clean reference analysis must use a separate project and preserve Gradle checkout basename; timeout cleanup must gracefully cancel and kill identity-checked descendants across process groups. Implementer extracted cohesive Session/Experiment/ProcessMeasurement and language setup/edit helpers; CodeScene reports 10.0 for all four harness modules. Initial TS/C# matrices completed 189 records each; independent clean-reference audits are running. A 685-source StoryEngine copy analyzed successfully and its 20 warm body-edit samples are running. Saved desktop session revealed billing checkout at /Users/zsolt.gajdacs/Projects/Recurring-billing; verification prerequisites now pass with explicit inputs.

Measurement collection complete: 756 fixture samples (four routes x nine scenarios x21), 21 larger-corpus samples and 140 native-event samples. Independent tester verified all selected stage metrics complete and no RSS sampling errors. TS clean audits match all original hashes; Maven and Gradle exact clean references pass; C# graph semantics match but all nine clean audits differ in generated-import provenance (TASK-63). Raw JSONL compressed losslessly after checksum/decompression checks; evidence/freshness/combined-summary.json selects the canonical datasets. Full verification passed compile/Clippy/Rust/Python/TypeScript stages and is executing release acceptance/benchmarks; final gate still pending.

Final verification PASSED all 32 stages at target/verification/freshness-final-20261113/results.json; durable results and quality logs copied to evidence/freshness/verification. Independent reviewer reports no remaining blocking source findings; seven focused tests passed; all six source files score 10.0 and safeguard passed. Sonar intentionally inactive (no project configuration). Post-measurement native-usage-aware cleanup completed with no errors. Research oracles remain explicit: C# metadata differs (TASK-63), and large-corpus independent semantic equivalence is unavailable. No production watcher, compiler reuse or automatic-refresh behavior was introduced.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Promoted DRAFT-3 and completed the measured saved-edit freshness spike. Retained 777 CLI samples and 140 native-event samples, a reproducible separated harness, full stage/resource evidence, typed-state/ownership/cancellation/adoption recommendations and dependent delivery tasks TASK-60–63. C# timestamp-sensitive import provenance is a measured follow-up, not a suppressed failure; unmapped environment inputs remain unverifiable. All 32 full-verification stages and CodeScene pass; production refresh remains follow-up scope.
<!-- SECTION:FINAL_SUMMARY:END -->
