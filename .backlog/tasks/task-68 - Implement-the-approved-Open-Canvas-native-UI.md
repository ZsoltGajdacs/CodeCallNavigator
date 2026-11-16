---
id: TASK-68
title: Implement the approved Open Canvas native UI
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T07:56:00Z'
updated_date: '2026-11-16T09:34:00Z'
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
- [x] #1 Native Rune graph workspace follows the approved Open Canvas layout in system light and dark appearances with usable controls at minimum supported window sizes.
- [x] #2 Project picker supports opening a folder, recovering unavailable recent roots, switching bounded persisted recent projects, and accessing build selection and analysis actions through the existing cancellation and generation lifecycle.
- [x] #3 Left Files and right Inspector panes have matching accessible open and close controls; resizing, visibility and widths persist independently of graph navigation and legacy sessions restore safely.
- [x] #4 Independent floating graph and activity bars retain grouping, conditional direction, depth, zoom, Fit, pagination and all loading, error, cancellation and freshness actions without overlap.
- [x] #5 Anchored filter popover retains all code visibility, relationship and target resolution choices independently; immediate application, production-default Reset, dismissal and focus handling work.
- [x] #6 Inspector retains signatures, source and supported flow actions, caller and callee paging, relationships, shortest paths, architectural detail, diagnostics and provenance; existing navigation, layers, flowcharts and comparison remain reachable.
- [x] #7 Hover over any eligible visible node shows its own real source or group file list without changing selection, inspector, history or camera, preserving loading, origin, error, pin, resize and stale-request protections.
- [x] #8 Valid node focus changes animate common nodes, attached edge endpoints and camera coherently; incoming and outgoing branches preserve relationship meaning, zoom and click/double-click navigation semantics.
- [x] #9 Rapid navigation retargets displayed state without jumps or queued transitions; manual pan and zoom interrupt camera follow, and obsolete layout or source results cannot overwrite current content.
- [x] #10 Animated hit testing, culling, selection and source anchoring use the same displayed geometry; outgoing ghosts are noninteractive and generation changes cannot confuse node identities.
- [x] #11 System reduced motion is honored, idle animation scheduling settles, and release native dense-graph performance evidence shows no unacceptable regression against a recorded baseline.
- [x] #12 Feature parity inventory, native light/dark screenshots and motion recordings, test inventory updates, CodeScene checks and scoped UI unit/integration/release-performance verification with cleanup evidence are recorded; unavailable checks are explicit, following the user direction to run only strictly needed verification.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Native implementation is complete with no feature-parity omissions and all acceptance criteria supported by recorded evidence.
- [x] #2 Required behavioral, native visual, release performance, CodeScene and applicable Sonar verification is complete, with any unavailable checks explicitly reported.
- [x] #3 Documentation, verification inventory and artifact cleanup are complete; no prototype data or browser code is used in production.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Implement in committed slices: 1. Reconcile feature parity and preserve artifact references under current documentation policy. 2. Native shell with matching independent Files/Inspector pane toggles and cohesive toolbar/status modules. 3. Grouped anchored filter popover retaining defaults and independent semantics. 4. Project picker with bounded recent roots and model-owned file filtering. 5. Floating graph/activity surfaces and inspector action hierarchy retaining all workflows. 6. Desktop-owned shared displayed geometry for cards, routes, hit testing, culling and source anchors; stable endpoint-key/classification edge matching and crossfade ambiguous parallels. 7. Interruptible focus/camera transitions, manual interruption and reduced motion. Targeted UI compiler/unit/boundary checks per slice, CodeScene review and safeguard before each commit; final native visual/performance verification required for animation. Keep stale-view inert guards, cancellation and generation validation. Use canonical Backlog testing policy doc-12. Prototype is illustrative, never production dependency.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Plan reviewed against current code. Required refactorings: thin shell composition; separate status/action presentation; dedicated filter popover; model-owned recent roots/file filtering; one graph-motion owner with bounded outgoing state. ViewEdge has no provenance identity: do not add semantic contracts just to animate it. Match stable endpoints plus classification, crossfade ambiguous duplicate routes. Source bounds must resolve displayed node geometry by key.

Slice checkpoint: native shell/pane controls, project picker with bounded recents, model-owned file filtering, grouped filter popover/default reset, inspector hierarchy/source action, floating controls/activity, and initial shared graph-motion geometry implemented. UI unit suite target/verification/ui-4xawnfqk passed 134 tests; category cleanup passed. Runtime integration is running with existing TypeScript fixture dependencies. Independent review identified follow-up motion route/layer/rapid-ghost hardening; task remains In Progress. Documentation-only planning commit 309e4fa already recorded. User requested targeted verification rather than broad unrelated analyzer reruns.

Verification scope follows the user instruction to run only strictly needed checks: desktop UI unit/runtime/real-CLI integration, native offscreen Metal visual and motion captures, matching-input original-versus-candidate release UI performance, CodeScene reviews and per-commit safeguards. Broad unchanged analyzer/core/MCP suites, packaging and the full composite are intentionally excluded, not claimed passed. Sonar configuration is absent. Acceptance criterion 12 updated to make this authorized scoped-verification choice explicit.

Hardening slice: stable endpoint/classification route crossfades, animated architectural bands, bounded continuous exit ghosts, shared hit/culling/source geometry, memory-light departing-card snapshots, safe measured floating-control framing, explicit external/history camera ownership, real status/freshness feedback and modal/focus corrections. UI unit210 passed (esmmq1lw), UI integration all runtime and real-CLI groups passed (u8mv5w26), category cleanup passed. Native Metal light/dark/minimum/popovers/source/layers and rapid motion captures reviewed under target/verification/open-canvas-visual-20261116-planner; owned visual build removed after process proof. Baseline64c performance3/3 passed with same-input fingerprints under target/ui-baseline-64c7e00; candidate comparison pending. Two performance inventory entries now correctly require CLI executable. No production dependencies/core/CLI contracts changed.

Final renderer diagnosis: controlled alternating release runs exposed per-node intermediate GPU layers, not frame-timing noise. The edge-to-edge SplitView main-pane rounded mask intersected each existing card clip. A local shell Kit now removes only that redundant outer radius, preserving card masks and ordinary control radii. Explicit graph-world Spring::NONE also prevents a second inherited layout clock from lagging drawn cards behind shared geometry. Added a real Rune presented-rectangle regression and a deterministic dense compositor resource guard; final exact verification pending. Final modified source CodeScene reviews are 10.0.

Final controlled release comparison passed after compositor correction: three alternating baseline/candidate GPU pairs use the same saved input fingerprints and synthetic dense graph. Dense medians 9.063→8.683ms at1x (-4.2%) and13.173→12.885ms at2x (-2.2%); dense intermediate targets302→0. Saved medians -17.0% at1x and+2.6% at2x. No material dense regression remains. Reports: target/verification/performance-xgeksm6k/corrected-controlled-gpu. Exact final source CodeScene10.0 and precommit safeguard passed (4 eligible files). Formatting, inventory JSON syntax and diff checks passed. Native visual evidence uses offscreen Metal rather than physical-window display/refresh-rate certification; captures predate only the redundant outer-mask correction, while card clipping remains unchanged.

Final exact presented-card runtime regression passed against the corrected release binary. Corrected release compilation38.17s; six serial GPU trials29.573s; final exact geometry test0.021s. Candidate ownership-checked cleanup passed, reclaiming1,272,183,839bytes with no retained build/profile/temp. Full timing/identity/limitations evidence is target/verification/performance-xgeksm6k/final-performance-review.json. Native capture AC mapping is target/verification/open-canvas-visual-20261116-planner/README.md. Baseline diagnostic cleanup is the final outstanding housekeeping step.

Baseline diagnostic cleanup passed; detached diagnostic worktree removed after no-handle ownership proof. Reclaimed1,271,804,809bytes with zero retained owned build/profile/temp; reports preserved under target/ui-baseline-diagnostic-64c7e00/final-retained-evidence and final-cleanup-record.json. Observed cleanup command7.8s, worktree removal0.006s. All authorized verification and cleanup complete.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the approved native Open Canvas redesign on codex/development in committed slices: independent docked panes, project picker/recents, grouped filters, floating controls/activity, inspector hierarchy and any-node source previews. Shared displayed geometry drives smooth interruptible cards/routes/camera with reduced motion and stale-result protections. Thin presentation modules and bounded motion state keep responsibilities separate. UI unit210 and integration252 passed; the additional exact presented-card regression passed red/green and final release replay. Release performance3/3 plus three corrected alternating GPU pairs passed with no material dense regression. CodeScene reviews and per-commit safeguards passed. Native Metal light/dark/minimum/source/filter/layer/motion evidence recorded. Physical-window frame pacing and broad unchanged component/package/full suites were not run, following requested verification scope. Owned diagnostic builds/profile copies and baseline worktrees cleaned; evidence retained.
<!-- SECTION:FINAL_SUMMARY:END -->
