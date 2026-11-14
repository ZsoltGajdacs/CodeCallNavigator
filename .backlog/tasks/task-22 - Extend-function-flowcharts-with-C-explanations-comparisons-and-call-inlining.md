---
id: TASK-22
title: >-
  Extend function flowcharts with C#, explanations, comparisons and call
  inlining
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-09T20:07:00Z'
updated_date: '2026-11-14T12:02:00Z'
labels: []
dependencies:
  - TASK-21
references:
  - docs/function-flowcharts.md
documentation:
  - docs/plans/task-22-function-flowcharts.md
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-21 establishes source-derived Java and TypeScript flowcharts. Track the deferred capabilities separately so broader language coverage, explanatory views and interprocedural exploration remain visible without expanding the initial delivery.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 C# methods can open source-linked flowcharts with validated language-specific control-flow semantics.
- [ ] #2 Optional AI explanations remain anchored to chart steps and distinguish generated interpretation from compiler/source facts.
- [ ] #3 PR comparisons show before/after function flows with explicit revision and source provenance.
- [ ] #4 Optional recursive call inlining preserves method identities and uncertainty, detects cycles and enforces visible expansion limits.
- [ ] #5 Document and test the supported behavior and limitations of each added capability.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Planning-only handoff (2026-11-14): no implementation authorized or completed in this pass. See docs/plans/task-22-function-flowcharts.md for verified integration points, semantic boundaries, verification strategy and open AI decisions.

1. Deliver C# first: TASK-22.1 exports bounded Roslyn facts and records the upstream patch; TASK-22.2 normalizes/persists them and serves production CLI queries; TASK-22.3 validates cleanup and unsupported boundaries; TASK-22.4 enables and accepts the desktop/package workflow.

2. Independently deliver revision comparison: TASK-22.5 builds on completed TASK-58 to serve bound historical flow/source; TASK-22.6 displays before/after charts with explicit endpoint provenance. Use exact IDs and saved source. Hosted PR lookup/fetch and heuristic rename matching are outside this scope.

3. Independently deliver bounded expansion: TASK-22.7 owns shared call-site occurrences, completion semantics, ambiguity/cycle handling and budgets; TASK-22.8 exposes opt-in CLI/desktop expand/collapse. Do not duplicate graph algorithms in desktop or MCP.

4. Resolve explanation design in TASK-22.9 before provider implementation: execution route, credentials/configuration, outgoing context, initiation/disclosure, retention and anchor/schema budgets require an explicit decision. Then TASK-22.10 delivers validated generation and TASK-22.11 the optional UI. No provider is selected by this plan.

5. Each child owns its tests, documentation and quality evidence. Use real compiler/executable traces, CLI/Git subprocess tests, deterministic provider fixtures and native workflow acceptance as applicable; implementation requires full verification outside the sandbox with isolated profiling inputs.

6. Completion mapping: parent AC1 = TASK-22.1-.4; AC2 = TASK-22.9-.11; AC3 = TASK-22.5-.6; AC4 = TASK-22.7-.8; AC5 is required in every branch. Preserve parent ACs unchecked until actual delivery evidence exists. C# can ship without waiting for the other branches.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Investigated current core IR, Roslyn body/binding pass and CallAtlas JSON, Rust C# adapter, generation-bound flow/source queries, desktop flow jobs, Git artifact loader/difference, existing trace fixtures and patch/bootstrap contracts. Created TASK-22.1 through TASK-22.11 as unassigned To Do subtasks with 5-6 observable ACs and task-specific verification/quality DoD. C# has no exported flow inventory today; historical flow rendering and provider-backed explanations are not implemented by current comparison/query infrastructure. Findings are a planning snapshot, not runtime verification. Original five acceptance criteria remain unchanged.

Planning handoff complete: all 11 child records read back through the CLI; parent links, exact dependencies, existing references/documentation, 5-6 unchecked ACs and two DoD items per child verified. Independent ready entries are TASK-22.1, TASK-22.5, TASK-22.7 and TASK-22.9. ./verify-light.sh passed in documentation-only mode (target/verification/light-gotf5s9l/results.json); document links/paths and untracked Markdown whitespace checked. Returning the implementation parent to To Do/unassigned because this session was investigation/planning only. No parent delivery AC was checked and no runtime test or implementation completion is claimed.

User approved sequential implementation of all subtasks on a new branch, with per-step testing, maintainability/refactoring review and verified commits. Active branch: feature/task-22-function-flowcharts. Planning baseline committed as 6162d4f. Execute C# 22.1-22.4, comparison 22.5-22.6, expansion 22.7-22.8, then explanation design/generation/UI 22.9-22.11; preserve the explicit provider/data decision checkpoint.
<!-- SECTION:NOTES:END -->
