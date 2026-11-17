---
id: TASK-76
title: Exclude incidental hidden folders and explain freshness blockers
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T06:26:00Z'
updated_date: '2026-11-17T07:17:00Z'
labels: []
dependencies: []
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Automatic refresh for Recurring-billing is blocked by incidental skill symlinks beneath broadly recorded repository scopes. Required build inputs must remain observed, and genuine blockers need actionable explanations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Broad discovery and freshness skip dot directories and symlinks consistently, including directory hashes
- [x] #2 Explicit compiler/build-required hidden and linked inputs remain tracked with contextual failures
- [x] #3 Versioned policy preserves legacy artifacts and requests one manual refresh without clearing cache
- [x] #4 CLI MCP and desktop expose detailed deduplicated bounded blockers and recovery guidance
- [x] #5 Necessary focused regression verification requested by the user, CodeScene review and native diagnostic acceptance completed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consolidate typed input selection and versioned fingerprint policy. 2. Audit adapters and preserve required inputs. 3. Consolidate structured blocker diagnostics and desktop presentation. 4. Add real boundary regressions and compatibility coverage. 5. Reuse completed selector results and run only necessary focused verification, CodeScene review and native acceptance as explicitly directed by the user; record limitations honestly.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared scoped_v1 selection and hash policy with typed broad/required roots, lexical required-link tracking, legacy policy migration guidance, structured actionable blockers, deduplication/bounds and selectable scrollable desktop details with Copy. Added real filesystem and Java/C#/TypeScript compiler regressions and native capture hooks. Initial selector compiled but stopped on lint unused import; fixed. Full rerun uses existing fixture dependencies and copied profile source. Sonar inactive: no .sonar/config.json.

Verification iteration: Rust and Node unit tests passed after updating the TypeScript scoped-policy capability fixture. Integration then found 3 versus 4 Next boundary edges with Flashcards dependencies; fixture pins TypeScript 6.0.3/Next 16.4.2, so switched final full selector to the existing matching PacificFateMapEditor node_modules without weakening assertions. Read-only updated-engine query of Recurring-billing saved to target/verification/task76-recurring-freshness.json confirms actionable legacy-policy guidance, deduplicated link warnings, and 375 unmapped input limitations; project artifact/cache unchanged. CodeScene safeguard passed (41 checked files); .gradle unsupported, Sonar unconfigured. Native captures enabled in final selector.

Review correction: known source deletion/rename is observable absence, not unreadability; reports input_missing as stale/nonblocking and allows compiler reanalysis to accept or reject new membership while preserving prior graph on failure. Broken required aliases and unreadable scopes remain blocking. Preserve coverage.kind=observed_inputs: unmapped environment limitations stay Unverifiable with counts without redefining scope coverage. Migration scheduling also checks persisted policy directly, independent of bounded diagnostic lists. Native details commands use matching runtime engine and root mappings; exact-content dedup retains trigger-only blockers.

User explicitly narrowed final verification to only necessary steps. Focused run reused existing runner, inventory, process timeouts and owned-workspace cleanup without editing selector rules; exact selection/driver/logs retained under target/verification/task76-focused. All 52 stages passed: 50 Rust tests, 32 Node tests and 4 real automatic refresh cases (burst-latest, invalid-repair, generated-output, legacy-selection). Gradle output-alias regression fixed by excluding owned class outputs from authored dependency alias inputs; raw ownership spelling preserved. Native Metal captures inspected: long paths wrap, inner diagnostics scroll to final entry, Copy details reachable and clipboard assertions pass. Timings: setup 1.131s, compilation 129.645s, test execution 86.583s, cleanup 1.243s; 1,293,232,089 bytes reclaimed, zero retained build/temp bytes. Prior matching-dependency selector check/clippy/unit tests and Next boundary regression passed; full suite did not complete and was not restarted per user instruction. No packaging/deployment/installation claim. CodeScene safeguard passed for 44 eligible modified files; Gradle language unsupported and Sonar unconfigured. Independent review performed; later tester agent hit service usage limit, so parent ran and inspected actual test evidence.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented scoped_v1 hidden-directory/symlink exclusion with explicit required-input preservation and legacy migration guidance. Added actionable bounded diagnostics and selectable, scrollable desktop details with matching-engine verification command and Copy details. Observable deletion/rename remains refreshable; unobservable required inputs remain blocking. Necessary focused verification passed with native visual acceptance and cleanup; full-suite completion not claimed under user-directed reduced verification scope. Existing graphs require one manual Refresh with an updated build. Recurring-billing artifact/cache not refreshed; unmapped environment remains explicitly unverifiable. No installation or push performed.
<!-- SECTION:FINAL_SUMMARY:END -->
