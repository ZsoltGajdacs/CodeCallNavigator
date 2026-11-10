---
id: TASK-26
title: Improve dense graph canvas rendering performance
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T09:29:00Z'
updated_date: '2026-11-10T09:36:00Z'
labels: []
dependencies: []
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Large node and edge counts make graph interaction run at low FPS. Reduce production frame work while preserving navigation and connector meaning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Dense graph rendering performs measurably less frame work with reproducible before and after evidence
- [x] #2 Graph and flow connector geometry, selection, pan and zoom remain correct
- [x] #3 Focused runtime tests and source quality checks pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Measure dense graph frame CPU time and drawing primitive count in the real Rune runtime. 2. Replace fixed cubic tessellation with bounded screen-space subdivision, retaining connector endpoints, direction and dash semantics. 3. Verify geometry and graph/flow interactions, repeat the benchmark and run CodeScene review. Keep upstream dependencies unchanged.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner and explorer identified fixed 24-segment tessellation per cubic as avoidable work; Rune emits a primitive per segment. TASK-26 created for this request.

Added ignored release benchmark using production Rune view: 300 visible nodes, 6000 mixed cubic routes, zoom 0.16/0.20/0.24; 12 warmup and 60 forced pan frames per zoom. CodeScene baseline edges.rs and benchmark each score 10.0. Pinned upstream source verification passes. Sonar inactive because .sonar/config.json is absent.

Adaptive screen-space cubic subdivision implemented with finite-segment distance, depth cap and endpoint-tangent arrows. Straight-cubic regression failed first on old renderer (25 points versus 2), then passed. Nine geometry/route tests pass, including exact direct/dashed straight-route Shape equality. Benchmark median frame CPU time improves 33-40% and line primitives fall 60-71%; raw before/after and reproduction in evidence/performance/canvas/. Independent review found no actionable issues. CodeScene reviews score 10.0 on all modified files.

Independent testing: 125 release binary tests pass in serial and default parallel modes; benchmark ignored by default. Fixed one timing-dependent checkbox UI test to mutate state through Runtime::update so redraw is explicit. Native Metal graph and flow captures inspected, copied to evidence/performance/canvas/. Release build, package.py and strict deep codesign verification pass. All modified source files score CodeScene 10.0; diff and scoped rustfmt checks clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reduced dense graph and flowchart connector work using bounded screen-space adaptive cubic subdivision and stable endpoint-tangent arrows. Production-runtime benchmark with 300 nodes and 6000 edges shows 33-40% lower median CPU frame preparation and 60-71% fewer line primitives. All 125 release tests, native rendering captures, release packaging and signature checks pass. Evidence and reproduction: evidence/performance/canvas/README.md. GPU display FPS was not measured.
<!-- SECTION:FINAL_SUMMARY:END -->
