---
id: TASK-18
title: Prevent terminal-launched analysis from suspending Gradle
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T14:05:00Z'
updated_date: '2026-11-09T14:12:00Z'
labels: []
dependencies: []
documentation:
  - evidence/analyzers/terminal-launch/README.md
priority: high
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Launching CodeGraph through dev.sh inherits a controlling terminal. Analyzer subprocesses use separate process groups but inherit terminal stdin; Gradle input forwarding can receive SIGTTIN after BUILD SUCCESSFUL, leaving the app waiting indefinitely at source-set ownership. Live StoryEngine process and controlled PTY reproduction confirmed SIGTTIN with inherited stdin and normal exit with closed stdin.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One-shot analyzer subprocesses do not inherit interactive terminal input, while protocol workers retain their pipes.
- [x] #2 A controlling-terminal regression verifies background subprocess completion without SIGTTIN and preserves process-group cancellation.
- [x] #3 StoryEngine analysis completes under terminal launch with the user JDK; focused tests, Clippy and CodeScene checks pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve live diagnostic evidence and reproduce inherited-versus-closed stdin under a PTY. 2. Add a failing process regression using a controlling terminal. 3. Disconnect stdin for one-shot subprocesses while preserving worker protocol pipes. 4. Verify cancellation and real StoryEngine analysis under a controlling terminal; rebuild app and document the restart requirement.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Observed live dev app waiting in run_process; Gradle metadata was already exported and stdout reported BUILD SUCCESSFUL in 5s. Process 77084 was stopped (T), in a separate background process group with fd0 inherited from the app controlling terminal. Controlled real StoryEngine wrapper reproduction confirmed SIGTTIN in 5.459s with inherited stdin and exit0 in 4.232s with closed stdin. Added real controlling-PTY production-runner regression; reproduced failure before fix, then verified EOF/non-terminal stdin after the single production change to Stdio::null(). Worker protocol pipe unchanged. Five process/worker/cancellation tests and all-target Clippy passed; both source reviews score10 and two-file safeguard passed. Independent review found no issues. Fixed debug CLI completed StoryEngine under controlling terminal with user SDKMAN25.0.2-jbr in42.188s, publishing8136 symbols and11112 relationships to isolated cache. Debug executable and signed release app/ZIP rebuilt; packaging verified. User live analysis was not signalled or cancelled. Existing process requires restart to load fix. Evidence: evidence/analyzers/terminal-launch/README.md and JSON/log records. Changes uncommitted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed terminal-specific Gradle suspension by closing one-shot analyzer stdin while keeping cancellation groups and TypeScript protocol pipes. Reproduced SIGTTIN before fix and verified terminal-backed full StoryEngine completion afterward (42.188s debug, user JDK). Regression/worker/cancellation tests, Clippy, CodeScene and package verification passed. Rebuilt debug and release artifacts. Current running user process remains untouched and must be restarted to pick up the fix; changes remain uncommitted.
<!-- SECTION:FINAL_SUMMARY:END -->
