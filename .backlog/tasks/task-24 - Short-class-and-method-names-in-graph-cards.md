---
id: TASK-24
title: Short class and method names in graph cards
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T08:50:00Z'
updated_date: '2026-11-10T08:59:00Z'
labels: []
dependencies: []
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Qualified names crowd graph cards. Implement approved short labels and visibility across Java, C# and TypeScript while preserving identities and inspector/search.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Graph labels use short names and visibility prefixes with descriptive hover/accessibility text.
- [x] #2 All three analyzers retain declared visibility including defaults, nested and combined access.
- [x] #3 Stable grouping, overload selection, lambda labels, file grouping and storage round trips are verified; caches refresh.
- [ ] #4 Tests, compilation, packaging and CodeScene checks are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add typed visibility and centralized projection formatting. 2. Extract metadata in Java, C# and TypeScript and invalidate caches. 3. Verify real fixtures, persistence and graph UI. 4. Review maintainability and quality checks, record results and finalize.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded approved scope. Confirmed a failing UI regression: class method row displayed Center::work(1) instead of work. Added shared typed metadata, SQLite columns and projection-only formatting; Java/C#/TypeScript extraction and real fixtures implemented. TypeScript 46 tests passed; Java focused acceptance passed. Initial combined UI run: 115 passed, new storage-to-UI selection fixture needs adjustment. Sonar inactive: no .sonar/config.json.

Review found no blocking defects. UI fixture initially placed its renamed class offscreen; qualified all fixture containers consistently and verified both overload clicks plus class/method accessible help. Full default Rust suite passed; Python 49, TypeScript 46, TypeScript real pipeline 9, C# acceptance 3, Java focused suite 24 passed. Release build passed. Packaging rejected unvalidated vendor/rune source changes. Maintainability follow-up: visible_node owned both group resolution and method presentation; two inline conditional variants tripped complexity gates, so split collapsed_group resolution from method projection. Split long storage-to-UI test setup into visibility_runtime.

Final review clean. Shared label formatting and group resolution refactors reviewed independently. Final Clippy -D warnings, fmt and diff checks passed. Final affected suite: 116 native UI/unit + 30 projection + 20 layer projection tests passed; storage core also passed. CodeScene modified source review completed; introduced issues resolved and affected files re-reviewed at 10.0. Final safeguard passed (27 eligible modified files, no regressions); new untracked source files separately reviewed. CallAtlas vendor retains unrelated existing CodeScene findings. Packaging gate remains unresolved because tools/package.py rejects unvalidated vendor/rune changes; task left In Progress, AC4 pending. No commit or push. One external Recurring Java acceptance requires CODEGRAPH_COMPILED_METADATA and was excluded from focused Java run.

User requested committing the verified implementation. Rechecked diff whitespace and CodeScene safeguard: passed, 27 eligible modified files with no regressions. Packaging limitation remains recorded; committing source changes without pushing.
<!-- SECTION:NOTES:END -->
