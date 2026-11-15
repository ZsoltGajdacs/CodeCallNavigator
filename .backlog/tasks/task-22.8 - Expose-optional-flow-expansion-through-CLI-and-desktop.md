---
id: TASK-22.8
title: Expose optional flow expansion through CLI and desktop
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:50:00Z'
updated_date: '2026-11-15T23:13:00Z'
labels: []
dependencies:
  - TASK-22.7
references:
  - crates/codecallnavigator-core/src/query/contract.rs
  - crates/codecallnavigator-core/src/query/operations.rs
  - src/flow_workspace.rs
  - src/flow_view/details.rs
  - src/ui_tests/function_flows.rs
documentation:
  - docs/plans/task-22-function-flowcharts.md
  - evidence/flowcharts/expansion/desktop/README.md
parent_task_id: TASK-22
type: feature
ordinal: 22080
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Publish the shared expansion result through the CLI and consume it in desktop interaction. Keep expansion decisions and source ownership in the CLI/core; UI owns expand/collapse state, layout and navigation. The existing single-function Flow request remains the default behavior and reference for collapsed output. An MCP tool is not required to deliver this desktop capability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An opt-in versioned bounded CLI request produces an expanded chart with source/occurrence ownership; invalid paths, stale generations and unsupported targets fail explicitly, while legacy single-function requests retain their behavior.
- [x] #2 Desktop supports expand/collapse at a selected call site and explicit candidate selection, shows method boundaries and cycle/limit reasons, and keeps source highlighting tied to the selected occurrence's file.
- [x] #3 Expansion is cancellable background work; obsolete replies cannot replace another function/generation, and collapse restores the equivalent unexpanded flow.
- [x] #4 CLI subprocess and native UI fixtures verify multi-file expansion, repeated callees, recursive cycles, budget exhaustion, stale replies and Back/camera/scroll behavior.
- [x] #5 Documentation and full verification record expansion bounds and semantic limitations; source Code Health, layout/performance measurements and change-specific visual acceptance accompany delivery.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Research query transport/generation/cancellation and desktop navigation/source ownership against current core expansion contracts. Use an explicit Expand calls mode: first request an addressed zero-selection chart, then expose selected call/candidate controls; ordinary Flow stays unchanged.
2. Add a version-1 FlowExpansion CLI operation requiring expected_generation, bounded selections/limits, cached exact source ownership and in-builder cancellation. Validate failures and preserve legacy query behavior through unit plus real subprocess coverage.
3. Add desktop expansion state and CLI adapter. Support explicit target selection, individual subtree collapse and collapse-all back to the saved ordinary session; show boundaries/stops and occurrence-owned source. Preserve history/camera/scroll and reject obsolete results across request/function/generation changes. Cancellation must reach owned query/layout work.
4. Verify production interactions in headless real-boundary tests and representative native light/dark minimum-window journeys, including multi-file/repeated/cycle/budget/source/navigation cases. Record layout/performance evidence and classify new tests with requirements/rationale.
5. Review and refactor each coherent unit, run CodeScene and full categorized verification outside sandbox with isolated inputs and cleanup. Update docs/evidence/Backlog, commit verified TASK-22.8 and stop before TASK-22.9.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-15: User authorized research, planning and implementation of TASK-22.8. Verified clean working tree at ea8aa02 and removed explicitly requested TASK-22.7 stash backup b634dd9. Continue existing feature/task-22-function-flowcharts branch; stop after this task is verified and committed, before TASK-22.9.

Research decision: expansion mode is explicit so ordinary charts need no structural-address inference. Zero-selection core ExpandedFlow supplies exact step_origins; selection clears on mode entry, individual collapse removes requested subtree, collapse-all restores saved ordinary session. Planner/explorer roles consulted.

Unit 1: New wire/capability tests failed for expected absent operation before implementation (target/verification/task22-8-wire-red). Root layout cancellation red attempts: r1 stopped on concurrent new-test inventory drift, fixed explicit registration; r2 compilation passed but process-ownership test could not enumerate ps inside macOS sandbox, so new layout assertions were not reached. Both category cleanups passed; rerun outside sandbox as r3. No failed or unrun checks are counted as behavioral proof.

CLI/core checkpoint green: 178 core unit and 113 core integration tests passed, including real Graphviz cancellation/reap regressions; all cleanup passed. Focused real public CLI saved-artifact batch passed 27 cases, with portable source owners, exact target/cycle/limits, generation/version and complete escaped 24 MiB response bound. Prior test-premise failures preserved. Reviewer cleared request validation-before-artifact-decode and Graphviz source changes. Unit source CodeScene reviews scored 10; both safeguard variants passed. Desktop tests-first implementation begins.

Desktop checkpoint: real .NET/CLI/Graphviz Rune regression reached expected missing flow-expand-mode control (target/verification/task22-8-ui-entry-red); cleanup passed. Native probe source review strengthened exact root call-site identity and visible highlighted source text checks; Code Health refactor keeps all native probe files at 10. Core/CLI source owners are portable @repo paths, unlike ordinary hydrated flow files; docs and native fixture matcher record this distinction. Desktop is currently locked; unlock request sent while independent work continues.

Desktop implementation: explicit mode, exact per-call candidate edits/collapse, independent source owner+digest mounts, saved ordinary restoration and generation/request publication guards. Reused comparison worker leases via shared background_work instead of duplicating ownership; app shutdown waits query/layout cleanup. Review found nested stopped-marker collapse targeting the caller; corrected marker precedence and explicit per-call collapse for shared markers. Added ten pure controller/capability tests using actual core expansion of in-memory IR, plus one real SDK/CLI/Rune journey. Native dispatch was temporarily held with exact hashes solely for missing-control runtime red, then restored unchanged (target/task22-8-progress/native-dispatch). First post-wiring compile found an existing direct Session test literal; switched it to the ordinary constructor. User unlocked desktop; owned caffeinate keeps idle display awake for upcoming native checks.

Final controller regressions: two real in-memory open_flow/Back tests reproduced canceled pending history retaining a ghost mode/status; typed pending intent and history normalization fixed them. A captured action guard now binds full choice/subtree, Ready/result identities, selected step and epoch/serial/generation; stale controls cannot resolve against mutable indices. Explicit first-load Cancel was separately reproduced and now restores the exact ordinary snapshot, fit and source scroll. All 15 focused pure cases passed with cleanup (task22-8-ui-policies-final). Real SDK/Rune r2 reached the strict minimum-window check and exposed Fit at x=1139.98 beyond width 1050; header split into native-inset navigation and shrinking title/action rows. The unchanged 27-case UI group is rerunning as r3; native/full verification still pending.

Minimum-window follow-up: strict actual/target Rune rectangles isolated inherited outer-root spring motion; scoped Spring::NONE on the FlowView root passed the unchanged six-frame geometry regression. Same-owner publication now retains its measured source mount/scroll handle (runtime red then green). The real SDK journey next exposed changed-file highlighting off-screen despite correct file/span/text; Rune applies center before absolute scroll offsets, so expansion restoration must choose one intent. A new real Rune two-source, equal-geometry regression is running before that focused correction. Native/full gates remain pending.

Source-scroll correction verified: actual Rune source-switch red consumed the mount but failed to realize selected row60; mutually exclusive centering versus saved-offset restoration fixes it. All three headless regressions passed with cleanup (task22-8-ui-source-switch-green). The real compiler/CLI/Graphviz/Rune expansion journey and 26 ordinary/C#/comparison neighbors all passed (task22-8-ui-focused-green-r5). Reviewer approved the correction. Final dual Code Health found only policy-test helper argument/assertion-size findings; production source passed, and behavior-preserving test-helper cleanup is underway before packaged native/full gates.

Final focused freeze: 15 pure controller policies and three real Rune source/header regressions passed with cleanup after helper-only maintainability refactoring; production and real SDK journey remain unchanged from the passing 27-case r5 run. Both CodeScene variants pass all 67 reviewed source files (65 score 10, two declaration-only) and safeguards; all 70 executable/test/inventory/deletion hashes match target/task22-8-final-verified/source-sha256.json. A transient IntelliJ file handle caused one owned cleanup failure; failure was preserved, fresh ownership/inactivity checks and production recovery reclaimed 2,754,669,045 bytes with zero retained. Focused/quality records copied to evidence/flowcharts/expansion/desktop; packaged native acceptance is now starting.

Packaged native acceptance passed 8/8 stages outside sandbox in both themes at 1050x680, with all 15 per-theme assertions, exact cached source after copied-source deletion, repeated occurrences/cycle/collapse/navigation and cleared query processes. Root inspected both owned-window PNGs: titlebar/toolbar controls fit, Helpers.cs line55 is visibly highlighted and explicit call controls remain available. Six first-Ready Drawn samples per theme observed 75.15–109.82ms for query/layout edits and 17.65–17.73ms for retained collapse-all; these are observations, not guarantees. Native cleanup passed, reclaiming owned scratch while preserving allowed packages. Reports and measurement scope copied to evidence/flowcharts/expansion/desktop. Mandatory full verification is now running outside sandbox with copied private Maven input and isolated profiling.

First full attempt stopped at Clippy: four new core test slice clones and large Side::Ready caused by adding inline expansion Mode to common Session. Runtime categories were not executed (77 stages passed, one failed, 40 not run); all always-cleanup stages, private-original fingerprint and source hashes passed, with the owned private copy reclaimed. Raw first-full stages/log/private-cleanup record retained in evidence/flowcharts/expansion/desktop. Corrective ownership decision: box optional Mode at Session rather than change unrelated comparison Side; use borrowed single-item slices in tests. Producer/reviewer are checking this bounded delta before a fresh full-r2 run.

Clippy correction is verified: only five files changed, optional Mode boxed at Session owner, comparison pair representation unchanged, four single-item test slices borrowed. Four core input/wire cases and 16 pure comparison/expansion/lease/capability cases passed; exact strict workspace all-target Clippy passed in the owned isolated category, with both cleanups passing. Reviewer approved. Both final CodeScene variants pass 68 reviewed files (66 score10, two declaration-only; pair.rs unchanged review-only), safeguards pass and 71-path hash freeze matches. Raw correction record is evidence/flowcharts/expansion/desktop/clippy-correction.json. Fresh mandatory full-r2 is running outside sandbox with separately named source/private/console reports; first attempt remains untouched.

Final delivery verified: full-r2 passed 118/118 stages with zero not-run selections (1,476 unique test registrations and 12 gates, not an inferred framework passing-test count). Recorded stage seconds: compilation 357.488, setup 10.432, execution 1457.376, cleanup 48.670, other 127.507. All five category/check cleanup reports passed and removed paths are absent; only allowed prior-package artifacts retained. Private original fingerprint is unchanged, owned copy removed, and all 72 before/after/current full-run hashes match; 71 reviewed-source paths also match. Root inspected six final light/dark native captures for ordinary, expansion and restored comparison flows. Corrected expansion first-Ready Drawn edits observed 96.92-118.71ms, collapse-all 16.80-17.82ms; representative observations, not worst-case guarantees. Direct staged-source CodeScene MCP safeguard passed with 67 eligible files checked. Only Backlog/evidence metadata changes follow this guard; executable/test bytes remain frozen. Mark all AC/DoD verified and stop after local commit, before TASK-22.9.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered opt-in version-1 generation-bound CLI expansion and desktop Expand calls/Expand here/Collapse here/Collapse this occurrence/Collapse all/Cancel controls. Core owns structural choices, conservative boundaries and exact cached source ownership; desktop owns bounded background interaction/layout/navigation. Ordinary Flow and comparison behavior remain compatible.

Verified real CLI, compiler, Rune and native boundaries. Corrected full verification passed 118/118 stages outside sandbox with isolated profiling and private Maven inputs, including performance, packaging and all cleanup. Both source CodeScene variants passed (68 reviewed files including unchanged pair.rs); the direct staged-source MCP safeguard checked 67 eligible files with no issues. No required gate waived; Sonar is intentionally inactive without project configuration. Evidence, failure/recovery chronology, timings and acceptance map: evidence/flowcharts/expansion/desktop/README.md.

Removed the explicitly requested stash backup. Preserve conservative source interpretation and documented budgets; AI tasks TASK-22.9 onward remain untouched. Commit this verified delivery locally and stop.
<!-- SECTION:FINAL_SUMMARY:END -->
