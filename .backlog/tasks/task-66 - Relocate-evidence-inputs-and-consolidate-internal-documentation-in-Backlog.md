---
id: TASK-66
title: Relocate evidence inputs and consolidate internal documentation in Backlog
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T07:02:00Z'
updated_date: '2026-11-16T07:18:00Z'
labels: []
dependencies: []
documentation:
  - >-
    .backlog/docs/planning/doc-1 -
    Repository-data-and-documentation-migration-plan.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
type: chore
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The evidence directory mixes generated output with required compiler baselines, while internal documentation is scattered across evidence and public docs. The user requested a plan before implementation: preserve executable inputs, put useful internal documentation in Backlog documents, retain external documentation under docs, and record this ownership rule in AGENTS.md. The researched proposal and complete docs classification are in doc-1; revalidate against current code when picking up the work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All required evidence inputs have explicit tracked owners outside evidence; relocated baseline bytes remain identical and their real consumers pass applicable checks.
- [x] #2 Every evidence Markdown source has a documented disposition; useful internal findings are consolidated into CLI-managed Backlog documents with historical context and limitations preserved.
- [x] #3 All current docs files are classified and split or migrated by audience; external commands, public contracts, schemas, installation and redistribution guides remain usable.
- [x] #4 Root and scoped AGENTS guidance links to canonical internal Backlog documentation and states the internal versus external documentation policy.
- [x] #5 Live references, task document metadata, anchors, output defaults and packaging consumers are updated; generated evidence is untracked and ignored while local outputs are retained.
- [x] #6 Applicable behavioral, documentation, packaging and Code Health checks are recorded, with unavailable checks explicit and unrelated work preserved.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve four baseline hashes and relocate required inputs. 2. Consolidate internal evidence/docs through CLI-managed Backlog documents and source dispositions. 3. Keep external contracts/guides stable, repair references and AGENTS policy. 4. Untrack evidence while retaining all local files and redirect generated outputs. 5. Complete independent review and focused/static light checks; full verification replaced by light scope at explicit user request.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User approved proceeding on feature/task-22-function-flowcharts. Initial tree contains only untracked approved Backlog plan/draft. Promoted DRAFT-1 to TASK-66. Sonar inactive: no .sonar/config.json.

Relocated four baselines with matching SHA256; 5 fixture-ownership, 2 engine-reference and 8 dependency-patch tests pass. Real raw C# export and installed-engine ingestion acceptance pass; real Maven corpus repeated/relocated artifacts match. Created internal Backlog history/docs/index and repaired task documentation metadata through CLI. Untracked evidence with git rm --cached, preserving local files. First full run stopped on unset private Maven variable; located existing /Users/zsolt.gajdacs/Projects/Recurring-billing in prior local evidence and reran with explicit inputs. All ten modified Python files initially Code Health 10.0; final prerequisites message review pending.

User explicitly replaced full verification with a light check. Sent SIGINT to owned full runner PID 48556; awaiting its normal cleanup. Full verification is waived for this task by user instruction; finish focused/static/light validation only. Reviewer requested sanitizing workstation-specific prose paths without touching frozen baselines.

Final review has no remaining actionable findings; workstation paths sanitized and root design report replaced with pointer. Light checks: 11 Python files parse, four JSON inputs parse, 315 local links/anchors resolve, whitespace clean; CodeScene all 11 source files score 10.0 and safeguard passes 11 eligible files. 44 focused tests pass, plus real C# export/ingestion and Maven repeat/relocation equality. Full runner stopped per user request; no owned runner/compiler processes remain. Unit category cleanup reclaimed 1459680432 bytes; broad cleanup retained pre-existing target/release (1146092741 bytes) after inconclusive lsof, without touching unrelated processes. Full is not claimed complete. All 929 original evidence files remain locally; Git tracks zero evidence paths. No commit/push. Detailed action/disposition/validation record: doc-33; local reports target/verification/task66-focused/.

User requested commit and push on the existing branch. Rechecked migration-only scope and whitespace; pre-commit Code Health safeguard passed 11 eligible files with no issues. No full verification was rerun.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Migrated four byte-identical executable baselines into fixtures; consolidated internal evidence, architecture and maintainer documentation in Backlog with a complete source disposition index; kept public schemas/install/release/CLI/MCP guides in docs. Added AGENTS ownership policy, repaired metadata/links and output defaults, and untracked/ignored evidence while preserving 929 local files. Independent review, 44 focused tests, real C#/Maven boundary probes, static documentation checks and CodeScene safeguard passed. Full verification was interrupted and replaced by light checks at explicit user request; retained existing release artifacts and incomplete cleanup are documented in doc-33. User subsequently authorized commit and push on the existing branch.
<!-- SECTION:FINAL_SUMMARY:END -->
