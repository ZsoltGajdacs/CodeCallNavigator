---
id: TASK-27
title: Emphasize selected graph node and direct connections
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T10:47:00Z'
updated_date: '2026-11-10T10:57:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dense depth-expanded graphs obscure the selected class because unrelated cards and connections retain full brightness. Preserve depth visibility while emphasizing only immediate connections.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Selected anchors and direct incoming/outgoing neighbors remain bright; other complete cards render at 35% opacity and remain interactive.
- [x] #2 Only edges touching anchors remain bright; other strokes and arrows use 8% of existing opacity and render beneath highlighted edges.
- [x] #3 Visible selection takes precedence over focus fallback; absent anchors restore normal brightness across graph views without changing depth or layout.
- [x] #4 Automated policy and production UI tests, theme inspection, compiler/lint, packaging and Code Health checks are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add a pure per-render GraphEmphasis helper with indexed anchor, neighbor and incident-edge membership. 2. Share it between card and edge renderers, centralize opacity, and paint dim edges first while preserving geometry. 3. Verify policy edge cases, depth and selection interactions using production UI runtime; inspect both themes. 4. Run Rust tests, clippy, packaging and CodeScene reviews; record evidence and finalize.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared per-render GraphEmphasis and dim-first edge rendering. Pure policy and Shape equality tests plus production class/method gestures verify adjacency, opacity inheritance, interaction and fallback. Independent review found no actionable issues. CodeScene all eight modified Rust files 10.0; safeguard passed 8 eligible files. Rust 270 passed/48 ignored; Python 50 passed; production Clippy passed. Strict all-target Clippy reports existing canvas_performance constant assertion lint; all-target run passed with only that lint suppressed through command line. Native dark/light captures at depth 2 and depth 5 inspected; final macOS packaging passed. Action log and evidence: evidence/ui/selection-emphasis/README.md. No source analyzer refresh, commit, push or PR performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Depth continues to control visibility; selection now emphasizes only its anchors, immediate neighbors and incident edges. Other complete cards use 35% opacity; other edge strokes/arrows use 8% existing alpha and paint underneath. Shared presentation helper keeps selection rules separate from traversal and geometry. Automated and native visual verification pass, with strict Clippy limited by the documented pre-existing benchmark lint. Final package: dist/CodeGraph.app.
<!-- SECTION:FINAL_SUMMARY:END -->
