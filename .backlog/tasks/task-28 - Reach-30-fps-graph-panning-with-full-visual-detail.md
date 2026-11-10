---
id: TASK-28
title: Reach 30 fps graph panning with full visual detail
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T11:47:00Z'
updated_date: '2026-11-10T12:43:00Z'
labels: []
dependencies: []
priority: high
type: enhancement
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Panning remained slow after TASK-26 reduced CPU tessellation. The saved 31-card/155-edge view emitted 14619 segments, including 9756 outside its clip. The user approved full-detail culling, pan-independent caches, native profiling and evidence-driven GPU optimization. Initial sandbox graphics restrictions were resolved with authorized Metal access; actual 2x GPU completion identified excessive line-quad overdraw.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Partly visible routes emit only needed connector pieces while preserving dash phase, arrows, styles and selection
- [x] #2 Panning reuses route geometry and emphasis; zoom, layout and selection invalidate correctly
- [x] #3 Native timing demonstrates at least 30 distinct camera presentations per second with nominal p95 interval at most 33.3ms for three ten-second drags on the saved and dense workspaces
- [x] #4 Saved and dense fixtures at 1x and 2x, regressions, Code Health and release packaging checks are recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve CPU and GPU baselines; establish native presentation measurement and signed binary identity. 2. Regress and implement padded per-piece clipping, analytic dash skipping, zoom/layout route caching and selection caching with explicit view replacement. 3. Measure dense 2x GPU cost, replace Rune AABB line vertices with oriented quads, capture the exact combined upstream patch and safe bootstrap migration, and verify differential pixels. 4. Run serial application tests, report and patch tests, 1x/2x CPU/GPU benchmarks, independent review, CodeScene gates, release packaging/signature checks and three ten-second native gestures per scene; retain reproducible evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner and explorer completed independent read-only checks. Native --script does not replay gestures; app-owned compositor will drive Runtime pointer events. Both native Metal and RUNE_GPU_BACKEND=wgpu report no graphics device. Initial culling regression failed as expected: 1819 dashes emitted for a 100-point viewport. Implemented padded segment clipping, analytic dash-prefix skipping, pan-independent route cache, and emphasis cache with explicit layout-replacement invalidation. Sonar inactive (.sonar/config.json absent). Native FPS acceptance remains mandatory and pending, not waived.

Native and GPU feasibility: authorized execution outside the sandbox found Apple M4 Pro Metal. System SQLite read-only WAL snapshots required a stable temporary DELETE-journal catalog for the runner; no live application data was intentionally written. Native scripts are headless-only, so an opt-in compositor queues production Runtime pointer gestures and records camera identities alongside actual Metal presentation timestamps. Python 3.9 streaming binary hashing, presentation-span FPS, missing/idle-frame rejection, live scale/dimensions and invalid GPU timestamp handling were verified.

Evidence-driven GPU unit: synchronous offscreen dense 2x median was 47.43ms (p95 51.33ms), with CPU update about 3.9ms. Tight oriented segment quads preserve the instance ABI and original fragment coverage. The combined pinned Rune patch includes prior warning fixes, shader and differential GPU test; bootstrap migrates only exact previous patches and rejects unrelated edits. Pixel comparison passed at 1x/2x within one RGBA8 LSB. Paired dense 2x p95 fell to 12.92ms; final checked-in benchmark repeat is 14.25ms. Repeated-draw textures and pacing changes were unnecessary after this improvement.

Final review/testing: independent reviewer found profiler denominator and overwritten GPU sample issues; both fixed with regressions and FIFO readback. Independent tester verified production gestures, clipping/caches/emphasis, reporter false-pass rejection and pinned sources. 141 release application tests pass, all three ignored performance benchmarks were run, 12 Python reporter tests and 2 real Git patch migration tests pass. All application/tool source is CodeScene 10.0; pre-commit gate passes (31 eligible files). Vendor metallib.rs remains 7.83 with identical existing findings; WGSL is unsupported by CodeScene and is covered by actual GPU compilation/pixel tests. Full upstream test-workspace dependency/release-cfg limitations are documented. Release packaging, strict deep codesign and exact patch checks pass; saved/dense/flow captures inspected.

Final native acceptance: monitor lock caused later retries to stop at two frames, correctly reported unavailable. macOS lock state was confirmed; user unlocked the desktop. Both final signed packaged-app runs then passed all three ten-second drags: saved 119.29-119.59fps with p95 8.334ms, dense 96.59-101.51fps with p95 16.668ms. Actual active window 1147x1344 points at 1x on 120Hz display; offscreen benchmarks use controlled 1900x1100 points at 1x/2x. Executable SHA256 353d491d204acf3060043b570354fff0326eca12a8c829acc30144b9fe025afc. Native 2x cadence requires a 2x monitor; only 1x is attached. GPU query zero/reversed pairs remain disclosed as incomplete native GPU samples; full offscreen GPU completion is measured independently. Evidence and full chronological verification detail: evidence/performance/panning/README.md.

Local source commits: 7cb1302 (viewport culling and pan-independent caches), 09c42ac (pinned oriented Rune quads and safe bootstrap migration), 473fb65 (native profiler and reproducible CPU/GPU benchmarks). No push or PR performed. Exact patch bytes are preserved by Git attributes. Native traces were reduced to exact timing fields and codegraph.* channels, with full-trace hashes retained; recomputing both retained reports passes.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented full-detail viewport connector emission, pan-independent geometry/emphasis caches and pinned Rune oriented line quads with safe patch migration. Native packaged-app panning reaches 119fps on the saved workspace and 97-102fps on 300 cards/6000 edges, with p95 presentation intervals at most 16.668ms. Dense 2x GPU-complete p95 improves from 51.33ms to 14.25ms in the final repeat. Tests, differential pixels, Code Health, packaging and signatures pass. Native 2x monitor cadence and exact OS mouse/drawable waits remain outside the available hardware/public-hook measurements; CPU/GPU behavior and pixels are verified at both scales. Evidence: evidence/performance/panning/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
