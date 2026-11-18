---
id: TASK-90
title: Hide unmapped-input warnings throughout the desktop UI
status: Done
assignee:
  - '@codex'
created_date: '2026-11-18T20:34:00Z'
updated_date: '2026-11-18T21:58:00Z'
labels: []
dependencies: []
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Routine unmapped portable inputs show an actionable-looking warning even when graph analysis and automatic refresh can proceed. Suppress that diagnostic throughout desktop presentation while preserving honest verification and genuine errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Unmapped-only observations with a valid contract, complete recorded coverage, and no omitted reasons show Graph ready with automatic refresh controls available.
- [x] #2 Desktop summaries, details, clipboard, and source popups omit unmapped-input warnings while retaining genuine blockers and unknown omitted reasons.
- [x] #3 CLI verification and structured freshness responses retain the original limitation, and refresh scheduling continues to use the original observation.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Centralize visible typed freshness reasons and neutral desktop status in the existing diagnostics module. Reuse the projection for the activity heading, summaries, source popup, details and clipboard.
2. Filter typed unmapped reasons before formatting coordinator blocker messages and preserve other blockers and omission counts. Keep wire responses and refresh decisions intact.
3. Extend existing regressions, run the deterministic verification plan and CodeScene reviews, and inspect the Recurring-billing desktop and CLI behavior.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Promoted the approved draft. Planner and explorer reviewed display surfaces, raw observation ownership, paired-engine compatibility and an isolated native acceptance route.

Reproduced the warning before editing with the existing desktop release, using the saved Recurring-billing graph in an isolated app store.
Implemented typed filtering for desktop diagnostics and coordinator copy. Independent review found no actionable issues with the paired build. Split the regression assertions following CodeScene feedback and confirmed all four modified source files scored 10.0.
The deterministic run passed compiler checks, Clippy, the MCP boundary check and unit suites. Updated paired binaries reproduced the expected CLI verification result and the original saved manifest hash.

Native Recurring-billing acceptance passed on the existing graph. Activity details retained the automatic refresh toggle and the pre-existing layer configuration warning while hiding the reported limitation. A separate Metal replay opened a real Java source preview without the warning. Evidence: target/verification/recurring-ui.dzsm5b/tester.
The first selector run passed 43 stages, including integration, 15 automatic refresh scenarios and the packaged C# desktop check. The light-theme flow-comparison journey failed its moved-ref Reload assertion after the dark-theme journey passed. Five later stages were skipped, and cleanup passed. Evidence: target/verification/changes-6zv5igm0. A complete unchanged retry is in progress.

All task acceptance criteria have evidence from the regression tests, paired native Recurring-billing UI and source replay, read-only CLI verification, and 15 automatic-refresh scenarios. Direct CodeScene reviews and safeguard passed for all four changed source files.
The complete selector retry passed 41 stages, then AeroSpace failed to recognize the owned packaged C# acceptance window. The same C# desktop gate passed in the first run, which stopped later at light-theme flow comparison. Both runs cleaned up successfully. Installed CLI, agent CLI/MCP journeys and native flow expansion never ran because of the earlier failures. The selector’s CodeScene stage also did not run, and the equivalent direct reviews and safeguard passed. Evidence: target/verification/changes-6zv5igm0 and target/verification/changes-eru67biu. TASK-90 remains In Progress until composite verification clears.

User explicitly approved closing and committing without completing composite verification. This supersedes the previous In Progress note. The two native gate failures and the later stages skipped by the selector remain recorded above.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The desktop suppresses unmapped-input diagnostics across its activity and details surfaces while retaining actionable warnings. Native Recurring-billing checks, unit and integration tests, all 15 automatic-refresh scenarios, and direct CodeScene checks passed. The user accepted incomplete composite verification after two native acceptance runs stopped at separate gates.
<!-- SECTION:FINAL_SUMMARY:END -->
