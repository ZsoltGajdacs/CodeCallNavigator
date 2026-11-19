---
id: TASK-96
title: Run unit tests during development and complete verification at finalization
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19T07:49:00Z'
updated_date: '2026-11-19T08:07:00Z'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up to TASK-72. This changes when required verification runs. DRAFT-1 separately considers narrowing the checks required by the complete plan. Development cycles should run only inexpensive unit tests. Before declaring a change done, agents must run every check selected by the existing changed-file policy and fix failures before finalizing.

Proposed implementation

1. Extend the existing verify-changes.sh entrypoint with an explicit development phase, invoked as ./verify-changes.sh --phase development --run. Reuse Git change discovery, inventory validation, existing unit stage builders, ownership, locking and cleanup. Compute the complete deterministic selection first, then execute only the selected unit registrations and their prerequisites, compilation and cleanup. For docs or boundary-only diffs with no selected unit registrations, report that outcome and the pending final checks without starting an empty build. Unknown or dependency changes can select all unit registrations during development and continue to require the complete full plan at finalization.
2. Keep ./verify-changes.sh --run as the final gate and preserve its existing selection and stage order. After the last edit, run all applicable checks, including units on the final tree, static and CodeScene checks, real compiler and integration suites, acceptance/native journeys and explicitly selected performance, package and release checks. Include the entire task diff using the appropriate --base when work already includes commits. Fix failures, use unit checks while iterating on the fix, then rerun the computed final plan. Preserve existing reporting for failures, missing prerequisites and deferred native checks.
3. Make report-only and executed output distinguish development evidence from final verification. Show final checks deferred by the phase separately from exclusions by policy and execution failures. Keep the final default command compatible, and include an explicit final command in development output.
4. Update root and scoped AGENTS.md and canonical testing/operations guidance together. Put behavioral combinations on production logic with explicit in-memory inputs and observable results. Keep policy and transformations separate from filesystem, SQLite, compiler, process and UI effects where the design has a natural boundary. Avoid mocked business behavior, fabricated analyzer outputs and test-only branches. Retain a real boundary check for behavior that depends on it.
5. Prove final plan equivalence for representative production, tooling, documentation, dependency/full-trigger and mixed diffs. Prove the development phase runs only selected unit cases and their necessary setup, handles no-unit plans and reports deferred final work. Exercise the tooling change with its complete computed final plan and record outcomes.

Acceptance criteria

- Agents can run a deterministic unit-only development plan through the existing entrypoint.
- Development success is labeled as such and cannot satisfy final verification. Report-only execution has no build or evidence side effects.
- The final entrypoint retains all currently selected coverage, including full fallback, and still reports failure and incomplete runs truthfully.
- Root and scoped guidance consistently require the final gate after the last edit, prioritize unit-testable production design and preserve real boundary coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Development runs only deterministic unit registrations and necessary build/setup/cleanup, with an explicit no-unit result.
- [x] #2 Development reports and evidence distinguish deferred final checks from policy exclusions and cannot satisfy final verification.
- [x] #3 The default final command preserves complete deterministic coverage, including automatic full fallback and truthful failure reporting.
- [x] #4 Root and scoped guidance require final verification and explain unit-testable production design with real boundary checks.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Keep changed-file rules and the final builder authoritative. Project the complete selected scope onto registered units for --phase development and run the ordinary shared unit category with required setup, compile and cleanup.
2. Keep the default final command and all final stages unchanged. Report actual development selection separately from checks postponed until final and policy exclusions, including a no-unit result without evidence allocation and clear development-only evidence.
3. Add focused planner/CLI/report regressions covering real Git, no-unit diffs, automatic full, and explicit-versus-default final parity. Register them in the inventory. Update root and scoped agent guides plus canonical Backlog testing and operations guidance.
4. Run development and complete computed final verification. Check CodeScene, compare report-only plans and validate truthful failures/deferred states. Record evidence and close TASK-96 once verified.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Promoted user-approved DRAFT-2 to TASK-96 on codex/development. Planner and explorer confirmed that development needs independent unit prerequisites and final planning must remain authoritative. The old CLI rejected --phase development. Implemented the unit projection, separate phase/deferred reports, and matching root/scoped and canonical guidance. Reviewer verified the workflow and later rendering extraction. Independent report checks and comparisons against the previous script matched all stages and output for the current focused plan (19 stages) and a real automatic-full plan (242 stages).

Development evidence target/verification/changes-jiri8lxf: 4/4 stages passed, 99 unit tests, about 9.6 seconds of recorded stages, and no final verification credit. The first final run changes-e720vl6t passed all behavioral stages and failed the CodeScene complexity gate in verify_changes.py. Extracting plan rendering resolved the finding. Final run target/verification/changes-zrqzb3h_: 19/19 stages passed, including cleanup and CodeScene. 365 tests passed. One existing optional TypeScript compiler journey was skipped because the tooling plan provisions neither the category-owned CLI nor TypeScript modules. The scope and historical skip behavior were retained. SonarMCP is inactive because the project is unconfigured.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added --phase development to verify-changes.sh for deterministic unit-only iterations and preserved the complete final plan as the default --run. Added phase-specific evidence, explicit no-unit outcomes and regression coverage. Updated root and scoped agent guides and canonical testing and operations documentation to prioritize directly testable production logic and real boundary coverage. Development and final verification passed, with the existing optional TypeScript tooling journey reported as skipped.
<!-- SECTION:FINAL_SUMMARY:END -->
