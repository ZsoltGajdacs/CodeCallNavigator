---
id: TASK-74
title: Include schemas with every analyzed graph
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T20:13:00Z'
updated_date: '2026-11-16T21:31:00Z'
labels: []
dependencies: []
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
External consumers need local graph contracts without network access. Approved scope: both schemas and a terse README on next successful analysis or refresh.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Successful publication includes both canonical schemas and a README under 200 words.
- [x] #2 Publication repairs companions; failures preserve the active manifest and companion bytes do not affect generation identity.
- [x] #3 Reads leave companions untouched.
- [x] #4 Fresh production output validates using adjacent schemas; verification and Code Health checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Embed canonical companions in core. 2. Atomically publish under existing guard before graph.json commit. 3. Extend filesystem and independent consumer checks and inventory. 4. Run deterministic verification and Code Health review.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented embedded canonical schemas and 154-word guide in shared publisher. Added filesystem repair/failure/read-only coverage and adjacent-schema validation to existing real CLI compiler journey. CodeScene reviews of six modified source/test files scored 10.0. Deterministic verification selected all consumer correctness and artifact performance checks. No separate red-test run: selector requires complete plan execution; coverage added alongside implementation.

First deterministic run target/verification/changes-b7_xcjyf passed static checks, cargo check/clippy and Rust/Python/Node unit suites; integration preflight stopped because corpus and TS dependency variables were unset. Located exact local paths in prior task73 verification evidence and restarted complete selector with those prerequisites. Cleanup stages completed on the stopped run. Independent reviewer found no actionable issues; pre-commit CodeScene safeguard passed for seven eligible files. storage.rs module-only file is unscored by CodeScene.

Second selector run changes-omnc8q6u passed compiler/lint/unit checks and progressed through real integration compiler fixtures. While active, the entire target/verification directory disappeared; runner exited 2 because results.json could not be written. Process inspection showed no remaining verification runner and a separate desktop cargo run, whose ownership was left untouched. This interrupted run does not prove complete verification or cleanup. Restarting full selector in a named task74 output directory.

Named verification run failed in real_cli_observes_repairs_and_owned_cancellation_completes: initial status Stale instead of Match. Root cause: generated_analysis_name did not recognize newly published companions, making the generated directory an observed input. Added exactly the three reserved names inside .codecallnavigator and extended existing real fingerprint tests to exclude generated companions while retaining same-named authored files at project root. Failure is direct red evidence; no timeout or assertion weakening.

Expanded generated-output exclusions for companion names and extended existing fingerprint integration cases; CodeScene scores 10.0 after extracting shared fixture resource names. Independent review also identified incremental_inference helper assuming every JSON file is a manifest; updating that assertion to distinguish schema companions from graph manifests. Complete verification restarted at target/verification/task74-schemas-fixed.

Fixed run passed all static/compiler/lint/unit/integration checks, including original freshness regression and all companion tests. Acceptance passed Rust journeys, packaging/signature/engine/MCP install, automatic refresh and C# cleanup/desktop/flow ingestion. Stopped at flow_comparison_desktop_acceptance because its PID/window ID never appeared in AeroSpace within existing 5s setup deadline. Cleanup passed. AeroSpace read-only inventory responds normally; no global state or timeout changed. One complete clean retry planned for this environmental window-registration failure.

User explicitly requested stopping verification, committing, and closing the task, stating: "close the task too, you have verified enough". Accepted exception for AC4: offline production-bundle consumer journey, later native comparison/expansion acceptance, and performance checks remain unverified because the earlier run stopped on AeroSpace registration and the final retry was interrupted at user request. No claim these checks passed. Final retry reports cleanup-unavailable for interrupted test execution; all category cleanup stages subsequently passed, reclaimed owned builds/tmp resources, and process inspection found no remaining task74 verification processes. Prior fixed run timings: compilation/package/checks 376.1s, setup 16.5s, test/gate execution 1439.9s, cleanup 44.1s. CodeScene safeguard passed for all 10 eligible modified source files; source reviews scored 10.0 (module-only storage.rs unscored). No Sonar configuration is present.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Published embedded graph/manifest schemas and a 154-word README with each graph bundle. Added atomic companion repair, read-only loading coverage, generated-output exclusions and independent consumer acceptance. Static/compiler/lint/unit/integration checks passed in fixed run; packaging and several real acceptance gates passed. Remaining gates explicitly accepted by user for closure; final retry stopped on request and owned cleanup completed. No push performed.
<!-- SECTION:FINAL_SUMMARY:END -->
