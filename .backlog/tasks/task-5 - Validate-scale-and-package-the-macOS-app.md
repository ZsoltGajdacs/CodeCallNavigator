---
id: TASK-5
title: Validate scale and package the macOS app
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-07T22:00:00Z'
updated_date: '2026-11-07T23:24:00Z'
labels: []
dependencies:
  - TASK-3
  - TASK-4
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Demonstrate the approved scale targets and ship a reproducible local Apple Silicon application with pinned dependencies and notices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Measure 250000 methods and 2 million edges against approved performance targets
- [x] #2 Package pinned analyzers layout tools licenses and reproducible Apple Silicon app build
- [ ] #3 Record headless interaction and live macOS validation evidence
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Run the 250000-method/2-million-edge benchmark; build and verify the relocatable Apple Silicon app with pinned analyzers, Graphviz and notices; exercise packaged analyzers and headless rendering; record live-display validation separately and leave it outstanding if unavailable.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Packaged dist/CodeGraph.app and ZIP, verified arm64 and deep strict ad hoc signature. Bundled fixture analyzer smoke passes with restricted PATH (25 symbols/26 relationships). Packaged real-cache GPU rendering and 331-frame interaction replay pass. Scale workload passes on available 48 GB M4 Pro: cached open 0.634s, search p95 106.1ms, neighbor p95 0.0078ms, peak 480MB. Exact 32 GB hardware remains unverified, so AC1 is left unchecked. Live native probe blocked: CoreGraphics reports zero active displays and display link -6661; AC3 remains unchecked. Detailed evidence and follow-up checks: evidence/VALIDATION.md. Implementation and artifacts are delivered; task stays In Progress for physical validation.

Final staged CodeScene safeguard passes: 45 eligible files, no issues. All introduced findings were refactored; all-target Clippy and all 34 Rust/20 Python tests pass after those changes. Rebuilt/signed package again, added omitted upstream Java license texts, and repeated packaged analyzer and 331-frame headless interaction checks successfully. No project apps or test-execution tasks were launched.

Final post-refactor 250k/2m benchmark passes: cached open 0.633s, search p95 99ms, neighbor p95 0.005ms, peak footprint 488440648 bytes. See evidence/performance/scale-final.json and scale-final-resources.txt. Resource measurement required unsandboxed kern.clockrate access; rerun completed successfully. Hardware remains 48 GB, not an exact 32 GB validation.
<!-- SECTION:NOTES:END -->
