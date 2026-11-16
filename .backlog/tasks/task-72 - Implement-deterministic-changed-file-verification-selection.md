---
id: TASK-72
title: Implement deterministic changed-file verification selection
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-16T17:51:00Z'
updated_date: '2026-11-16T18:14:00Z'
labels: []
dependencies: []
ordinal: 78000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents currently choose verification scope manually, and full/light classification does not map changed components to required checks. Make a deterministic changed-file selector the mandatory agent entrypoint, preserving real-boundary coverage and safe execution.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Changed-file rules deterministically select component correctness, performance or full verification with text and JSON explanations and no scope override
- [x] #2 Report-only mode has no build or evidence side effects; --run executes the selected plan through shared runner and cleanup
- [x] #3 Git changes, consumer expansion, inventory edits, unknown paths and deterministic full triggers have behavioral coverage
- [x] #4 Root and scoped agent instructions require verify-changes.sh for verification and test execution
- [ ] #5 Automatically selected focused tooling verification and applicable Code Health checks pass; tooling-only edits do not build application tests
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Implement explicit selection rules, robust Git discovery and report model. 2. Compose multi-component category selections through the shared runner and add verify-changes.sh. 3. Register pure policy/unions plus real Git, CLI and generated-plan equivalence tests. 4. Require the selector in all agent guides and canonical docs. 5. Validate using verify-changes.sh --run; verification-tooling edits select focused tooling coverage with no application builds. Full remains automatic for production dependency/bootstrap/packaging/vendor changes and unknown paths.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented versioned additive rules, NUL-delimited index/worktree/untracked discovery, base/index/worktree inventory unions, deterministic full fallback and report/execute entrypoint. Component unions propagate through preparation/prerequisites/execution with separate policy-exclusion reporting. Root plus all scoped AGENTS and canonical testing/operations docs now require verify-changes.sh. Independent planner/explorer/implementer/reviewer/tester roles completed via a delegated agent; review caught and fixed TSX quality omission, dropped command options, ambiguous Git inventory reads and staged-restored metadata. Automatically selected full run changes-nyck3164 is in progress; prerequisites/static/compiler/Clippy/boundary stages passed. New tests registered in the inventory. SonarMCP is inactive: no .sonar/config.json in this project.

User corrected the broad verification-infrastructure trigger: tooling-only changes have no application runtime effect and must select tooling tests plus generated-plan equivalence, not rebuild application integration tests. Stopped run changes-nyck3164 by SIGTERM to the verified lock owner. Its interrupted integration stage reported cleanup-unavailable; all category and final cleanup stages subsequently passed. It is not passing full-run evidence. Revised the checked-in rule, regression tests, AGENTS and canonical docs. CodeScene safeguard initially found complexity in two new modules; cohesive identity-comparison/report helpers resolved both to 10.0.

Final focused run target/verification/changes-dla9fr3r: all 29 stages passed; compilation 0.0s, setup 6.063s, tests 127.735s, cleanup 1.243s, other/static/quality 19.453s. Selected 345 tooling registrations: 344 executed successfully and one pre-existing CompilerJourneyTests.test_saved_edit_refresh_and_failure_recovery skipped because its CLI/TypeScript inputs were absent. The skipped test is not passing evidence; its existing tooling/integration inventory row has no requirements despite exercising a compiler journey. All 27 new tests passed. First focused run changes-vmw8kthw hit the pre-existing observer EOF shutdown race; it passed on rerun without changing or weakening that test. Category cleanup passed. CodeScene reviewed all 15 changed source/test files at 10.0; safeguard passed with no issues. Independent final review found no remaining selector defects; stale operations documentation was corrected. Content/path/diff and policy path-audit checks completed. No commit or push performed. AC5 remains pending the existing unrun compiler journey; implementation and AC1-4 are verified.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented deterministic changed-file verification selection, mandatory agent entrypoint, component unions and truthful policy-exclusion reports. Verification tooling now selects tooling checks without application builds; production dependency/bootstrap/packaging/vendor and unknown paths still select full automatically. All 27 new tests, all 29 final execution stages and CodeScene checks passed. Validation remains qualified by one existing compiler journey skipped for missing CLI/TypeScript inputs and an existing EOF race observed on the first run; the task remains In Progress rather than claiming complete selected-test coverage. Evidence: target/verification/changes-dla9fr3r/summary.json and change-plan.json.
<!-- SECTION:FINAL_SUMMARY:END -->
