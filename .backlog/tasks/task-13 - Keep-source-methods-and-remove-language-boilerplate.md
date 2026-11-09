---
id: TASK-13
title: Keep source methods and remove language boilerplate
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T09:18:00Z'
updated_date: '2026-11-09T09:34:00Z'
labels: []
dependencies: []
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Method graphs contain constructors, initializer scaffolding, inherited Object members and compiler-created helpers. Apply the approved source callable policy across Java, C# and TypeScript with separate discovery, policy and pruning responsibilities.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All languages exclude constructors, initializer nodes and Object signatures including overrides while preserving unrelated overloads and all visibility levels
- [x] #2 Java inventory excludes compiler helpers and preserves lambdas; C# exports compiler facts; TypeScript preserves React and Next behavior
- [x] #3 Shared pruning preserves identities, edges and entry metadata without reconnecting excluded nodes
- [x] #4 Old graph policies require refresh and valid empty analysis succeeds
- [x] #5 Real analyzer acceptance, relevant tests, packaging and CodeScene checks pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Separate compiler discovery, language policy and shared graph pruning. 2. Implement Java source inventory, C# Roslyn facts and TypeScript callable facts. 3. Version persistence and handle empty results/navigation. 4. Run real fixtures, independent review, tests, packaging and CodeScene.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer roles confirmed adapter facts, dedicated policy and shared pruning boundaries. Shared pruning tests first failed on missing module, then passed after implementation. Added persisted policy enforcement for graph loads and SQL search; removed empty graph publication rejection. Rust core/policy tests and focused library/UI/projection tests pass. CodeScene shared pruning, persistence, jobs and analyzer orchestration score 10. Language implementers are running real analyzer acceptance.

Independent reviewer found C# Object signatures on interfaces and explicit implementations survived; fixed using Roslyn Object type lookup and explicit interface symbols, with real regressions passing. Review also identified Java source-root scanning could include compiler-excluded files; implementer is restricting discovery to compiled provenance and testing no-debug fallbacks. TypeScript 20 adapter and 8 real pipeline tests passed; Rust all-targets passed; Python 33 tests passed. Initial package built and signed successfully; final artifact will be rebuilt after review fixes. Safeguard identified Java helper/test and TS facts complexity; targeted refactoring in progress (TS now 10).

Final independent reviews resolved all actionable findings. Java fallback now handles compiled-source selection, no-debug mismatched filenames, longest dollar-name ownership, package-info and stale deleted-source outputs. Verification: cargo test --all-targets passed; Python unittest discovery 33 passed; Node adapter 20 passed; real TypeScript pipeline 8 passed; real C# project and folder acceptance passed (project requires local MSBuild IPC outside sandbox); Java direct analyzer 5 passed; Maven/Gradle builds 8 passed; duplicate-context Maven/Gradle 2 passed; packaged Java 2 passed. Final tools/package.py built and codesign-verified dist/CodeGraph.app and ZIP. Changed Rust formatting and source whitespace checks passed; .patch context-space lines are diff syntax. Final CodeScene safeguard passed across 34 eligible files with no regressions. All new/modified Java helper modules and Java/Rust policy code score 10. Sonar inactive: no .sonar/config.json. Optional private recurring-billing acceptance was not run because its environment is absent; it is not required for this task.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed constructors, initializer scaffolding, compiler helpers and Object signatures across Java/C#/TypeScript graphs while preserving authored callables, unrelated overloads and generated-source visibility. Compiler facts, pure language policy and shared structural pruning have separate responsibilities. Persisted policy requires refresh of old caches and supports empty graphs. Real analyzer fixtures, standard suites, independent review, signed packaging and CodeScene safeguard passed.
<!-- SECTION:FINAL_SUMMARY:END -->
