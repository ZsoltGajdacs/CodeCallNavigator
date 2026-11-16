---
id: TASK-68
title: Implement the approved Open Canvas native UI
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-16T07:56:00Z'
updated_date: '2026-11-16T08:14:00Z'
labels:
  - ui
  - ux
dependencies: []
references:
  - evidence/ui/open-canvas-design/inspector-toggle.jpg
  - evidence/ui/open-canvas-design/filters-concept.png
  - evidence/ui/open-canvas-design/hover-source.jpg
documentation:
  - doc-34
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the approved native Open Canvas design from doc-34 and local evidence/ui/open-canvas-design. Preserve all existing workflows, any-node hover source, and independent right Inspector/left Files panes. Deliver smooth interruptible graph navigation in committed slices. The browser reference is illustrative only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Native Rune graph workspace follows the approved Open Canvas layout in system light and dark appearances with usable controls at minimum supported window sizes.
- [ ] #2 Project picker supports opening a folder, recovering unavailable recent roots, switching bounded persisted recent projects, and accessing build selection and analysis actions through the existing cancellation and generation lifecycle.
- [ ] #3 Left Files and right Inspector panes have matching accessible open and close controls; resizing, visibility and widths persist independently of graph navigation and legacy sessions restore safely.
- [ ] #4 Independent floating graph and activity bars retain grouping, conditional direction, depth, zoom, Fit, pagination and all loading, error, cancellation and freshness actions without overlap.
- [ ] #5 Anchored filter popover retains all code visibility, relationship and target resolution choices independently; immediate application, production-default Reset, dismissal and focus handling work.
- [ ] #6 Inspector retains signatures, source and supported flow actions, caller and callee paging, relationships, shortest paths, architectural detail, diagnostics and provenance; existing navigation, layers, flowcharts and comparison remain reachable.
- [ ] #7 Hover over any eligible visible node shows its own real source or group file list without changing selection, inspector, history or camera, preserving loading, origin, error, pin, resize and stale-request protections.
- [ ] #8 Valid node focus changes animate common nodes, attached edge endpoints and camera coherently; incoming and outgoing branches preserve relationship meaning, zoom and click/double-click navigation semantics.
- [ ] #9 Rapid navigation retargets displayed state without jumps or queued transitions; manual pan and zoom interrupt camera follow, and obsolete layout or source results cannot overwrite current content.
- [ ] #10 Animated hit testing, culling, selection and source anchoring use the same displayed geometry; outgoing ghosts are noninteractive and generation changes cannot confuse node identities.
- [ ] #11 System reduced motion is honored, idle animation scheduling settles, and release native dense-graph performance evidence shows no unacceptable regression against a recorded baseline.
- [ ] #12 Feature parity inventory, native light/dark screenshots and motion recordings, test inventory updates, applicable CodeScene and configured Sonar checks, full verification and cleanup evidence are recorded with unavailable checks explicit.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Native implementation is complete with no feature-parity omissions and all acceptance criteria supported by recorded evidence.
- [ ] #2 Required behavioral, native visual, release performance, CodeScene and applicable Sonar verification is complete, with any unavailable checks explicitly reported.
- [ ] #3 Documentation, verification inventory and artifact cleanup are complete; no prototype data or browser code is used in production.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Implement in committed slices: 1. Reconcile feature parity and preserve artifact references under current documentation policy. 2. Native shell with matching independent Files/Inspector pane toggles and cohesive toolbar/status modules. 3. Grouped anchored filter popover retaining defaults and independent semantics. 4. Project picker with bounded recent roots and model-owned file filtering. 5. Floating graph/activity surfaces and inspector action hierarchy retaining all workflows. 6. Desktop-owned shared displayed geometry for cards, routes, hit testing, culling and source anchors; stable endpoint-key/classification edge matching and crossfade ambiguous parallels. 7. Interruptible focus/camera transitions, manual interruption and reduced motion. Targeted UI compiler/unit/boundary checks per slice, CodeScene review and safeguard before each commit; final native visual/performance verification required for animation. Keep stale-view inert guards, cancellation and generation validation. Use canonical Backlog testing policy doc-12. Prototype is illustrative, never production dependency.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Plan reviewed against current code. Required refactorings: thin shell composition; separate status/action presentation; dedicated filter popover; model-owned recent roots/file filtering; one graph-motion owner with bounded outgoing state. ViewEdge has no provenance identity: do not add semantic contracts just to animate it. Match stable endpoints plus classification, crossfade ambiguous duplicate routes. Source bounds must resolve displayed node geometry by key.
<!-- SECTION:NOTES:END -->
