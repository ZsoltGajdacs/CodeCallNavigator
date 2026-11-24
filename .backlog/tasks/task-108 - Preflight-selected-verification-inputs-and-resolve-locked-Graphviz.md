---
id: TASK-108
title: Preflight selected verification inputs and resolve locked Graphviz
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T21:24:00Z'
updated_date: '2026-11-23T21:58:00Z'
labels: []
dependencies: []
type: bug
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A focused verification run rebuilt checks and unit suites before discovering missing final-suite inputs. An installed locked Graphviz can also be hidden by a newer Homebrew default.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Verification uses an installed Graphviz matching analyzers.lock.json when the default dot differs.
- [x] #2 All selected runtime inputs are checked together before focused final compilation, and an input-only command runs the same checks without building.
- [x] #3 Development and reporting retain the existing deterministic selection and side-effect boundaries.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Resolve the pinned Graphviz executable for verification subprocesses.
2. Preflight the union of selected requirements before build stages and expose the same step without a build.
3. Verify plan ordering, input failures, locked version selection, and compatibility.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified both preflight paths on this host. Missing fixture settings were reported together and the installed Graphviz 16.1.1 was selected automatically under a default 16.1.2 PATH. Preflight rejected a loaded TypeScript 6.0.3 compiler paired with a 6.0.2 manifest. Matching fixture inputs passed before the combined build began.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Focused final tooling verification passed all 22 stages, including generated-plan checks, input validation regressions, and CodeScene. Development verification passed all four stages. The completed reports are archived under target/verification-reports/task108-final-complete and target/verification-reports/task108-development-verified.
<!-- SECTION:FINAL_SUMMARY:END -->
