---
id: TASK-84
title: Integrate thin clients with continuous engine sessions
status: To Do
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-18T12:33:00Z'
labels: []
dependencies:
  - TASK-83
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
MCP and desktop must consume engine recovery rather than implement separate policy. Slice 6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MCP CLI and desktop expose equivalent engine outcomes and codes
- [ ] #2 Desktop opt-in works before first publication and cancellation stops sessions
- [ ] #3 All clients use the new engine contract directly; stale versions fail explicitly and packaging/native acceptance pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The final checkpoint must resolve the accepted locked-desktop deferrals from Slices 1–2 and TASK-86. In the latest matched run, changes-wmwd6yi3, desktop stage 61 was interrupted during fixture preparation before GUI launch or WindowLease. Stage 62 (real CLI ingestion with parallel cases), flow comparison, installed CLI, agent CLI/MCP journeys, flow expansion, performance/query/storage benchmarks and the late CodeScene stage did not run. Earlier common checks, Rust acceptance, automatic refresh, C# cleanup and category cleanup passed. Source Code Health and commit safeguards passed independently. Run these deferred checks with an unlocked desktop at final verification; scheduling protocol probes do not replace the real compiler journey.

Slice 3 replacement run changes-q97ej3qj passed 58 stages through automatic-refresh acceptance. The pre-window guard interrupted C# cleanup acceptance before native-window startup. C# cleanup acceptance and all later native windows, CLI ingestion, installed CLI, agent/MCP journeys, performance and late CodeScene checks remain pending for the final checkpoint. Category/global cleanup itself passed. User reconfirmed that the desktop will be locked and that these checks can wait until the end.

Slice 4 final exact run changes-6jx6ktlw passed 74 stages through all 15 automatic-refresh cases. The guard interrupted C# cleanup acceptance prior to desktop startup. C# cleanup acceptance, native windows, subsequent CLI and MCP journeys, performance and late CodeScene remain part of the final checkpoint. All category cleanup reports recorded zero errors.
<!-- SECTION:NOTES:END -->
