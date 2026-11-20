---
id: TASK-78.1
title: Verify graph snapshots using build evidence
status: Done
assignee:
  - '@codex'
created_date: '2026-11-20T11:02:00Z'
updated_date: '2026-11-20T16:43:00Z'
labels: []
dependencies: []
parent_task_id: TASK-78
type: bug
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Incidental output in a Maven module can thwart Git attribution despite leaving the analyzed graph inputs untouched. Use compiler evidence through a shared contract implemented by Maven, Gradle, C#, and TypeScript.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All four adapters supply target-owned evidence. Shared retention uses language-neutral rules.
- [x] #2 Prepare Snapshot tolerates unrelated artifacts with the Recurring Billing UI enabled and rejects unverified consumed inputs.
- [x] #3 Input mutations preserve the prior graph and saved snapshots stay readable.
- [x] #4 Real fixtures, deterministic verification, Code Health, and packaged Compare prove the change.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Introduce normalized evidence and shared publication and Git checks.
2. Populate evidence in Maven, Gradle, C# and TypeScript.
3. Cover regressions, document the extension contract and run verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Traced the reproduced failure to broad Java module directory fingerprints. Added per-target evidence, moved dependency and analyzer metadata decisions to their adapters, split scheduling watches from publication, and added real Gradle, Maven, C#, and TypeScript regression coverage. Independent review led to checking dependency aliases against tracked paths, requiring consumed evidence for each selected target, and shipping real fixture prerequisite fixes. Development passed at changes-ru5q3wa6 after correcting a test setup and formatting failure. CodeScene safeguard passed all 31 eligible files. The optimized private snapshot probe and final selector are in progress. Read-only freshness is tracked in TASK-78.2.

Optimized engine retained Recurring Billing commit 3e975f1f3d1e with the frontend enabled, snapshot 85942475c867. CLI comparison loaded the stored snapshot and reported equality. The full run changes-ta3kh8ah caught a legitimate Gradle producer rewrite incorrectly guarded before javac. The Java adapter now lets the compiler journal own consumed source membership and bytes while guarding separate declared producer inputs. Development changes-4qeqipgs passed. Final run changes-0ob44luh is in progress.

Subsequent integration runs exposed existing compatibility contracts. Maven no-source operations now validate Java source membership alone, preserving missing-output handling without launching Maven again. TypeScript context validation now joins the existing one-worker lifecycle. The C# conditional-import test asserts the project directory as a watch while retaining its typed import-change check. Full run changes-aasbv_zt passed Rust, Python, and Node integration and reached the raw CallAtlas shape gate. CallAtlas compiler evidence now uses an engine-only opt-in to preserve the default raw graph projection. The patched resource rebuilt without warnings or errors.

Full run changes-parsshu1 passed all new snapshot fixtures and reached native Compare. The empty-commit comparison differed only in Comparison.AssemblyInfo.cs provenance, because the SDK injected SourceRevisionId. Disabled the informational version stamp in the comparison fixture to keep its compiler inputs identical. Two isolated packaged-engine preparations of distinct commits then produced the same verified graph digest, 44d980fb3966. Final selector rerunning on the corrected fixture.

Final verification changes-vdhz6sb3 passed all 90 stages without deferrals, including packaged native Compare, all four adapter fixtures, installed CLI and MCP, performance, CodeScene, and cleanup. The packaged engine retained Recurring Billing commit 3e975f1f3d1e with frontend builds enabled as snapshot 34197c949e3e. Isolated private-project and Node fixture copies were reclaimed after the run and after confirming process inactivity. See target/verification/changes-vdhz6sb3 and target/verification/recurring-snapshot-packaged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Maven, Gradle, C#, and TypeScript now produce per-target evidence from evaluated builds and compiler inputs. Shared snapshot verification checks exact input content and revision attribution. Unrelated output is excluded from the proof. Real fixtures verify input-change rejection, preservation of the prior graph, and saved snapshot compatibility. All 90 final verification stages passed. Read-only freshness remains tracked by TASK-78.2.
<!-- SECTION:FINAL_SUMMARY:END -->
