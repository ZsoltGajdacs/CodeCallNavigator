---
id: TASK-107
title: Align Compare node labels and source previews with the normal graph
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-23T21:06:00Z'
updated_date: '2026-11-23T22:16:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compare used qualified names and signatures where the normal graph showed short labels. Compare hover previews also closed before users could interact with the source.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Compare labels share the normal graph formatter and preserve detailed names and CLI compatibility.
- [ ] #2 Compare hover source previews use the graph popup for pointer entry, scrolling, resizing, pinning, and dismissal.
- [ ] #3 Revision-bound data, cancellation, native behavior, and required verification are covered.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Share label formatting and carry optional saved naming facts.
2. Share the graph popup lifecycle and panel with revision-bound Compare previews.
3. Verify native scenarios, code quality, and CLI compatibility.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shared the desktop label formatter and the graph source popup, including clamped placement and the dismissal grace period. Added additive revision-bound display metadata, preserved existing wire fields, and extended native acceptance for pointer entry, scroll, resize, pin, Escape, and unchanged cameras. The first unit build exposed one old pinned-state fixture, and the next run exposed a comparison fixture that changed visibility on one side while asserting a flow-only change. Both fixtures were corrected. Per-file CodeScene review passed after refactoring popup placement and event handling.

Development, Cargo check, Clippy, selected Rust integration, and the changed-source CodeScene reviews passed. On the run under target/verification/changes-2koijioe all integration stages passed, including the Compare catalog, normal graph popup, and mixed Java/TypeScript recovery tests. Acceptance prepare stopped on reports copied from another checkout with foreign ownership markers. The reports were archived outside the workspace discovery tree and ownership was revalidated. The rerun under target/verification/changes-h3k832mo passed the relevant Compare catalog and normal popup tests again, but the existing mixed Java/TypeScript recovery test timed out waiting for the UI to settle. Native Compare and performance stages remain unverified. TASK-108 independently passed its full tooling plan.
<!-- SECTION:NOTES:END -->
