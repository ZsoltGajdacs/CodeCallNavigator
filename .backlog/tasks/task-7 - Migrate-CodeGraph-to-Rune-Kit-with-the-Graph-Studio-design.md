---
id: TASK-7
title: Migrate CodeGraph to Rune Kit with the Graph Studio design
status: Done
assignee:
  - '@codex'
created_date: '2026-11-08T21:01:00Z'
updated_date: '2026-11-08T21:23:00Z'
labels: []
dependencies: []
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The custom controls and fixed panels make graph exploration crowded and inconsistent. Deliver the approved Graph Studio concept using Rune Kit while preserving existing analysis and navigation behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Rune Kit shell and controls replace custom standard controls through focused components and a shared light/dark theme
- [x] #2 Activity rail opens repository, paginated search and filters; resizable collapsible drawer and inspector leave all existing graph controls reachable at minimum size
- [x] #3 Existing graph semantics, analysis flows, navigation, keyboard shortcuts and inspection remain functional
- [x] #4 Presentation preferences persist independently of graph navigation; old sessions load and pane changes preserve camera
- [x] #5 Runtime tests, Rust and Python checks pass; fixture and real-project screenshots cover dark/light and default/minimum sizes
- [x] #6 Component review and applicable Code Health checks pass with documentation and validation evidence recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Inspect existing Rune view/state/session boundaries and pinned Kit APIs.
2. Add presentation state and compatibility tests; introduce shared CodeGraph Kit theme.
3. Migrate controls and focused drawer/graph/inspector components; compose SplitView shell and rail.
4. Verify keyboard, overlays, resize/camera behavior and saved preferences with real Rune runtime tests.
5. Capture and inspect fixture/real screenshots in both appearances and sizes; run Rust/Python and Code Health checks; finalize evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented Rune Kit shell, focused drawer/graph/inspector components and shared palette. Legacy-session and presentation tests pass. Reviewer found max-pane overflow; added width budgeting and runtime drag regression. Visual QA found fixed-width input overflow and truncated Namespace label; corrected responsive input widths and grouping allocation. Clippy and 27 Python tests passed; full Rust suite and screenshot matrix in progress.

Final validation: 73 Rust tests passed including compiler-backed ignored regression; 27 Python tests passed; Clippy warnings denied passed. 26 UI runtime tests include 14 new Graph Studio scenarios. Reviewer re-review found no material issues. CodeScene reviewed components/tests at 10.0; safeguard passed16eligible files. Sonar inactive (no config). Captured and inspected eight fixture/real PNGs and trees at default/minimum sizes in both appearances; source/full-frame/focused comparisons and design-qa.md passed. Testing additionally caught and fixed the pre-existing clipped empty-canvas prompt. Physical trackpad/window-shell and VoiceOver use remain unverified. Evidence and chronological QA corrections: evidence/ui/graph-studio/README.md and design-qa.md.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Migrated CodeGraph to the approved Graph Studio layout using the pinned Rune Kit shell and standard components. Added isolated, backward-compatible presentation preferences, responsive pane budgeting, drawers, themed graph colors and focused inspector sections. Preserved analysis/navigation semantics. All acceptance criteria verified through runtime tests, full Rust/Python checks, Clippy, Code Health and native Metal visual QA. Source, comparison captures and check logs are recorded under evidence/ui/graph-studio; physical shell/trackpad and VoiceOver interaction were not exercised.
<!-- SECTION:FINAL_SUMMARY:END -->
