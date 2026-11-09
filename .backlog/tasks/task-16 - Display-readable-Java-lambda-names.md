---
id: TASK-16
title: Display readable Java lambda names
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T11:46:00Z'
updated_date: '2026-11-09T11:59:00Z'
labels: []
dependencies: []
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Java lambda nodes expose compiler names instead of authored names. Implement the approved readable Java lambda plan across shared analysis and symbol lists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Named Java lambdas show field/local names and anonymous lambdas show context and locations when unambiguous.
- [x] #2 Ambiguous and no-debug lambdas receive deterministic readable fallbacks without guessing source identities.
- [x] #3 IDs, signatures, relationships, filtering and source navigation remain unchanged; labels persist and are searchable.
- [x] #4 Graph, search and relationship lists show friendly names and retain original signatures for inspection.
- [x] #5 Compiled Java, persistence/search, UI and packaging checks pass; required Code Health checks are reported.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extend and version Java syntax-tree inventory. 2. Resolve names conservatively in shared normalization and invalidate cached graphs. 3. Update symbol lists and fixture search. 4. Verify compiled fixtures, persistence/search and UI. 5. Review, package and run Code Health checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded approved plan after planner and explorer review. Reproduced original behavior with a failing compiled acceptance test (COST_TYPE_VALIDATION missing). Extended Java AST inventory and shared label normalization; same-line/nested overlaps intentionally fall back. Java adapter rebuilt successfully. Reviewer identified legal authored lambda$ method names; gated naming on synthetic flag and added regression coverage. Sonar inactive because .sonar/config.json is absent.

Compiled coverage now passes for named fields/locals, inline callbacks, ambiguous nested and same-line lambdas, no-debug fallbacks, direct assignments, cast/parenthesis wrappers, nested classes, and legal authored lambda$ methods. SQLite round-trip/search passed; existing authored-lambda path/filter tests passed. Two production Rune UI tests cover graph/search/neighbor/relationship labels and signature inspection. Full default cargo test --all-targets passed; Clippy passed. Rebuilt Java installDist, packaged Maven/Gradle resource tests (2) and python3.13 tools/package.py passed. Final CodeScene review: all 16 modified source files score 10.0; pre-commit safeguard passed with 16 checked, no regressions. Broader Java build/duplicate-source-set regressions still running. No commit or push performed.

Final independent reviewer found no remaining actionable issues after synthetic-flag fix. All 8 java_builds ignored regressions passed (Maven profiles/reactors/cancellation/cache and Gradle classification). Formatting and final no-deps Clippy checks passed. App artifact: dist/CodeGraph.app; distributable: dist/CodeGraph.zip. Duplicate-class/source-set regression suite is finishing.

Final verification complete: both Gradle and Maven duplicate-class/context regressions passed (2 tests); final lambda suite passed all 7 including compiled debug/no-debug fixtures. All acceptance criteria verified. Changes remain uncommitted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Java lambda labels now use direct source field/local names or readable contextual locations, with conservative fallbacks for ambiguous/no-debug methods. Names are shared across Maven/Gradle/UI/CLI, searchable and persisted; IDs, signatures, graph relationships and navigation remain intact. Symbol lists show friendly labels and inspector retains signatures. Versioned inventory and graph policy require refresh for existing workspaces. Verified full default test suite, 7 lambda checks, 4 existing compiled Java checks, 8 build regressions, 2 duplicate-context regressions, 2 packaged-resource tests, UI tests, Clippy, formatting, app packaging and Code Health (all 16 source files 10.0; safeguard passed). No remaining review findings; no commit/push. Built dist/CodeGraph.app and dist/CodeGraph.zip.
<!-- SECTION:FINAL_SUMMARY:END -->
