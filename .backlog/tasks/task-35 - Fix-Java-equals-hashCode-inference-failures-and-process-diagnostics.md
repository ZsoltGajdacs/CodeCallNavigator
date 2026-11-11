---
id: TASK-35
title: Fix Java equals/hashCode inference failures and process diagnostics
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T06:18:00Z'
updated_date: '2026-11-11T06:41:00Z'
labels: []
dependencies: []
documentation:
  - evidence/engine/stages/java-equality-inference-2026-11-11.md
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Recurring-billing reaches the upstream snapshot cap in equals/hashCode; cancellation reports an ambiguous timeout error. User approved an exact-signature inference exception with strict analysis elsewhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Exact equals(Object) and hashCode() skip inference but retain raw declarations/calls; every other method keeps full inference and completeness checks.
- [x] #2 Cancellation and timeout have distinct contextual errors and lifecycle regressions.
- [x] #3 Real-JVM regressions, documentation and graph diagnostics cover the policy.
- [x] #4 Rust, Java, CodeScene, packaging and full Recurring-billing validation are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add Java inference regressions and exact-signature policy plus docs/diagnostics. 2. Separate process cancellation/timeout with lifecycle tests. 3. Rebuild and validate Java, Rust, Code Health, packaging and full Recurring-billing. 4. Review changes and record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Java regression reproduced the old inference behavior, then all 11 real-JVM context tests passed after exact-signature switch. Added bounded regression deadlines after independent review. Process errors and 10 lifecycle tests implemented by backlog-implementer; independent review found no blockers. CodeScene reviews initially flagged five arguments in process helper, refactored to ProcessLogs and re-reviewed: all six affected source/test files score 10.0. Sonar inactive because .sonar/config.json is absent. Broader validation and copied full-project analysis underway.

Full copied Recurring-billing Maven reactor succeeded in220.04s and published10827 declarations/17331 relationships across13 contexts. Original saved failing context explicitly reports analysisComplete=true, engineSucceeded=true and frameSnapshotLimitMethods=0. Engine packaging succeeded. Required desktop check/tests, engine check/tests and strict workspace Clippy passed. Longer errors exposed unread pipe backpressure in an integration test; concurrent drains fixed the harness and all4 protocol tests pass (CodeScene10). Independent reviewer found no final blockers; CodeScene safeguard passed7 eligible files. Exact debug include-ignored command hit3 existing release-only canvas benchmarks. Full functional rerun excludes only those benchmarks and supplies the temporary private reactor path; still running.

Final validation complete: both private Recurring-billing acceptance tests passed. The compiled-output acceptance needed CODEGRAPH_COMPILED_METADATA; exported real successful Maven test-compile metadata from the temporary copy, then all 67 engine tests passed with include-ignored. All desktop/core functional targets passed in the workspace run (459 successful tests before its missing-metadata stop); remaining engine coverage passed on rerun. All 3 canvas benchmarks passed separately in their required release profile with copied workspace and Metal. Final strict workspace Clippy passed. Evidence includes commands, reproduction, initial environment/profile failures and successful reruns. Changes remain uncommitted; no push or PR requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the exact equals(Object)/hashCode inference exception, retaining raw declarations/calls and strict inference/completeness elsewhere. Cancellation and timeout diagnostics now include operation, elapsed time and log paths; timeout stderr reads are bounded. Added real-JVM and subprocess regressions, corrected protocol test pipe draining, and documented policy. Full copied Recurring-billing reactor published 10827 declarations/17331 relationships over 13 contexts in 220.04s; saved failing context reports zero snapshot-limit methods. Java, Rust functional and engine acceptance, release benchmarks, strict Clippy, CodeScene and engine packaging verified. See evidence/engine/stages/java-equality-inference-2026-11-11.md for commands, logs and profile/fixture prerequisites. No commit or push.
<!-- SECTION:FINAL_SUMMARY:END -->
