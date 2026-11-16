---
id: doc-34
title: Open Canvas native design contract
type: other
created_date: '2026-11-16T08:14:00Z'
updated_date: '2026-11-16T08:14:00Z'
---
## Design contract

The selected direction is the light Open Canvas concept, refined through the interactive prototype. Use `inspector-toggle.jpg` for the latest panel controls, `hover-source.jpg` for source previews, `filters-concept.png` for the filter organization, and `inspector-concept.png` for the inspector hierarchy. The checked-in `prototype/` is a frozen browser interaction reference, not production code or an analyzer fixture. It contains illustrative code and relationships. Recreate it with native Rune/Kit, preserving real graph semantics and existing app features. System light/dark appearance remains supported; the approved light palette guides the light theme, not a light-only restriction.

Approved behavior:

- Project picker at top left; file explorer on the left; inspector docked on the right. Matching side-oriented toolbar toggles and pane-header close controls. Both panes remain independently resizable and closable, restoring saved visibility and width.
- Spacious graph canvas. Grouping, depth, direction where applicable, Fit, zoom and pagination are in a floating graph-control bar. Activity/loading/freshness lives in a separate floating bar; bars must not overlap or conceal required controls.
- Filters open in an anchored popover: code visibility, relationship kinds, and target resolution remain independent groups with all existing choices. Immediate application; Reset restores documented production defaults, not the prototype's all-checked behavior. Changed-state indicator compares with those defaults.
- Inspector prioritizes selected identity/signature, source location and source/flow actions, then callers/callees. Relationships, shortest directed path and analysis details remain available through disclosures, with paging and certainty/provenance intact.
- Every eligible visible node offers source on hover regardless of selection. Preserve existing group file listings, source loading/errors, live/analyzed origin, stale-location warnings, pin/resize/copy behavior and cancellation. Hover does not select, move the graph or rewrite the inspector.
- Clicking a different node highlights it promptly. After the existing single/double-click decision and valid asynchronous layout completion, common nodes move smoothly, outgoing nodes fade, new nodes appear, and the camera follows without forced zoom resets. Rapid navigation retargets from the displayed state.


Native implementation is TASK-68. Approved captures and the browser prototype remain local references under evidence/ui/open-canvas-design, excluded from version control. The native app retains real source/graph behavior; prototype data is illustrative.
