---
id: TASK-93
title: Preview changed-symbol impact from one retained graph
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-18T22:02:00Z'
updated_date: '2026-11-19T07:16:00Z'
labels: []
dependencies:
  - TASK-91
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Map changed base lines to symbols and follow callers without a head compilation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A cached baseline gives a potential-impact preview with no compiler run.
- [ ] #2 Missing and unresolved coverage are explicit in all interfaces.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Resolve an exact retained baseline and compare its tracked old-side Git ranges against a requested revision or the tracked working tree, with no compiler run.
2. Map ranges to baseline symbols, walk incoming call relationships, and report unmapped changes and bounds explicitly.
3. Expose the same read-only request and report through CLI, MCP, and desktop. Add focused Git and parity coverage and run the repository selector.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented CLI, MCP, and desktop impact preview with one retained baseline, a single bounded Git diff, saved source spans, caller evidence, and explicit unavailable coverage. CodeScene safeguard passed on 42 eligible files. Selector compilation, Clippy, and unit tests passed, including the three impact tests. Isolated real TypeScript probes confirmed a mapped body line, its caller, explicit insert-only coverage, and identical CLI and MCP reports with analyzer resources unavailable at preview time.
<!-- SECTION:NOTES:END -->
