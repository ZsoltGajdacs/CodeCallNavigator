---
id: TASK-23
title: Implement centered call-tree traversal and class drill-down
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T22:34:00Z'
updated_date: '2026-11-09T22:49:00Z'
labels: []
dependencies: []
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Single-click dimming and detached class expansion make walking relationships difficult. Build on TASK-6 and TASK-10 with centered caller/callee traversal and progressive exploration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single-click centers a filtered caller-above/callee-below neighborhood using depth on both sides; double-click creates one drill-down history step.
- [x] #2 Every visible class lists scrollable methods; method rows and class-header drill-down reach method trees with external context.
- [x] #3 Traversal is deterministic and bounded, preserving cycles, history, legacy sessions and stale-result protection.
- [x] #4 Interaction, rendering, navigation and projection responsibilities are separated; tests, CodeScene, visual review and documentation are complete.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Baseline and refactor interaction/rendering. 2. Focus targets, membership and signed-rank traversal with anchor paging. 3. Class rows, drill-down and shared layout completion. 4. Behavioral, Graphviz and UI coverage, review, packaging and documentation.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Baseline cargo test --all-targets --offline passed non-ignored tests. Planner/explorer established aggregate-first traversal and typed focus targets. Implementation split across projection, rendering and centralized interaction/layout completion. Sonar inactive: no .sonar/config.json. Initial CodeScene reviews: navigation, interaction, app and jobs 10.0; simplifying one layout conditional before re-review.

Implementation: extracted graph rendering from camera, centralized gesture/navigation actions and shared layout completion, introduced typed persisted focus with legacy compatibility, aggregate-first bidirectional traversal, class method rows and internal class trees, qualified Layer tab handling, variable Graphviz geometry and anchor-prioritized paging.

Review/fix loop: addressed explicit camera input during pending layouts, footer resize centering, invalid-focus overview fitting, class internal ordering, filter-overlay-aware Fit, exactly-1500-method paging priority and asynchronous Layer assignment restoration. Independent reviewer reports no remaining blockers.

Final validation: cargo test --all-targets --offline passed 246 tests with 46 toolchain/fixture tests ignored; terminal-isolation test required escalation for ps. Both packaged Java resource smoke tests passed using real Maven/Gradle/JDK artifacts. Python 49 tests passed. Clippy all targets with warnings denied, project-only rustfmt and diff whitespace checks passed. Every modified source file reviewed with CodeScene; final safeguard passed 31 eligible files. Sonar inactive: no .sonar/config.json. Six native Metal captures inspected in dark/light and minimum-window Fit states; evidence and logs in evidence/ui/centered-traversal/README.md.

Packaging limitation: fresh bundle provenance check rejects pre-existing ignored vendor/rune local edits; preserved dependency checkout. Packaged-resource compatibility itself passed both smoke tests. Physical trackpad/VoiceOver not verified. No commit, push or app installation performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented centered caller/callee traversal, progressive class/method drill-down and scrollable method signatures, with focused refactorings and preserved history/session behavior. Final Rust, Python, packaged-resource, Graphviz/native interaction, Clippy, formatting and CodeScene checks passed. Visual evidence is recorded; fresh app bundle remains blocked by pre-existing Rune source-provenance changes. Changes are local and uncommitted.
<!-- SECTION:FINAL_SUMMARY:END -->
