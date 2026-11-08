---
id: TASK-6
title: Add Method view and directional top-down layouts
status: Done
assignee:
  - '@codex'
created_date: '2026-11-08T08:14:00Z'
updated_date: '2026-11-08T08:20:00Z'
labels: []
dependencies: []
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Method exploration currently requires expanding structural groups and the left-to-right Both layout scatters related methods. Add entry-rooted method exploration with predictable caller/callee order and maintainable component boundaries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Method beside Folder opens all entry candidates and highlights verified application starts; focus and Overview work.
- [x] #2 Callers and Callees define deterministic top-down branch grouping in all views; legacy Both becomes Callees.
- [x] #3 Traversal, scope, navigation, ordering, pagination and geometry have focused responsibilities preserving limits and counts.
- [x] #4 Analyzer, Graphviz, native UI, legacy compatibility and regression checks pass; usage documentation updated.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Baseline tests and separate analyzer/projection/UI implementation.
2. Export/persist candidate and verified-start identities with legacy fallback.
3. Shared multi-root traversal, explicit scope, deterministic branch order and vertical layout.
4. Method control/highlighting, centralized transitions and legacy direction migration.
5. Independent review, Rust/analyzer/Graphviz/UI tests, Code Health and documentation.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Baseline cargo test --locked passed: 33 passed, 1 ignored JDK integration. Applying Backlog task-loop and Code Health safeguards. Sonar inactive: no .sonar/config.json.

Baseline passed; new multi-root tests first failed for the missing API. Shared traversal now uses MultiNeighborhood request and stable identity BFS with a 10.0 Code Health score. Navigation extracted into focused module; UI Method/direction/highlighting and legacy migration coverage added. Analyzer metadata and projection/layout implemented in parallel under separate file ownership.

Completed Method UI, entry metadata and verified-start highlighting; added legacy cache/direction compatibility and safe pinned analyzer patch upgrade. Projection now separates selection, aggregation/pagination, traversal order and geometry; navigation transitions live in navigation.rs. Independent reviewer identified Java class initializer candidates; excluded them and verified with real BCEL fixture.
Final validation: cargo test --locked -- --include-ignored: 52 passed, none ignored (real .NET/JDK fixtures included). Python unittest discover: 27 passed, including 9 bootstrap cases. Clippy all targets with warnings denied passed. Project-only rustfmt check passed. Code Health safeguard passed, 24 eligible files checked, no issues; touched production files scored 10.0. Source diff whitespace check passed excluding canonical unified patch context lines. Sonar inactive because project has no .sonar/config.json.
Rendered and visually inspected production Graphviz method-callees and method-callers scenes: top-down branches, shared methods once, recursion retained, real arrows preserved. Preview PNGs: /tmp/codegraph-method-callees.png and /tmp/codegraph-method-callers.png. Independent final review found no blockers. Changes remain local; no commit or push performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented entry-rooted Method view, verified application-start highlighting, two-direction top-down layouts with deterministic branches and shared-method cross-links, focused projection/navigation modules, and safe legacy analyzer/session/cache upgrades. Verified with 52 Rust tests including real analyzers, 27 Python tests, Clippy, formatting, Code Health and real-render visual review.
<!-- SECTION:FINAL_SUMMARY:END -->
