---
id: TASK-64
title: Refresh vendor source tips and integrate pinned patches
status: Done
assignee:
  - '@codex'
created_date: '2026-11-13T18:46:00Z'
updated_date: '2026-11-13T19:33:00Z'
labels: []
dependencies: []
documentation:
  - evidence/verification/vendor-refresh-2026-11-13.md
type: chore
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User requested the latest Git vendor source changes after committing the hook research handoff, with integration, verification and a separate commit on the same PR branch. TASK-41 records the earlier November 11 refresh and its historical validation limitations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Git source vendor pins match fetched upstream default branch tips and changes are reviewed
- [x] #2 Applicable local patches and migration checkpoints are rebased, checksum-pinned and bootstrap reproducible
- [x] #3 Integration passes full verification and applicable quality checks, with precise evidence recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify current vendor diffs against recorded patches and fetch default tips. 2. Review upstream changes, rebase patches and checkpoints, update locks and resolve integration changes. 3. Run focused checks and full verification outside sandbox. 4. Review and run Code Health safeguards; commit vendor integration separately, push branch and open PR.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Research handoff committed separately as 3938d24 on codex/vendor-refresh-hook-research. Fetched all three source upstreams; exact current patches verified. Only Rune advances bba30a61 to c4eda058 (338 commits); CallAtlas and java-callgraph2 unchanged. Full verification prerequisites confirmed available using existing TypeScript modules, private Maven checkout and a saved-app profile source (benchmarks will use a snapshot). Integration requires explicitly enabling Rune tooling and consuming its upstream workspace patch overrides.

All four Rune alternatives rebased and verified in isolated checkouts (clean apply, exact diff, checkpoint migration and reverse-clean). Obsolete focus method hunk removed; observers moved into new browser startup and kept before direction-chord/autoscroll early returns. GPU regression adapted to new loader API without changing shader bytecode fingerprints. Added upstream integration changelog. Root enables tooling and selects upstream patched crate paths. Workspace all-targets cargo check passed with upstream warnings. A pre-existing migration fixture assumption failed on the newly added changelog; fixed to seed old-side paths and compare whole checkout including untracked added files. All 7 dependency tests and independent migration checks passed. CodeScene test-file review 10.0; rebased source reviews reported existing upstream complexity, no finding in newly added callback statements. Independent static review has no actionable findings.

Full ./verify-full.sh --base origin/main passed all 27 stages (target/verification/full-e45ldvgw/results.json): ordinary Rust 674 passed/110 ignored; release 784 passed/0 failed/0 ignored; 193 Python tests; tools suite 50 with one optional compiler journey skipped, then explicitly run and passed; 112 TypeScript tests; bootstrap, Clippy, storage and CPU/GPU benchmarks, packaging/signatures, installed CLI and independent MCP/compiler journeys passed. Both private Recurring Billing cases passed. Focused upstream Rune GPU pixel regression passed. Packaged dark/light captures compared with the previous package: existing filter/card overlap is present in both; no layout fix added. Upstream compilation warnings and stale precompiled-library fallback remain explicitly documented, not claimed warning-free. Evidence: evidence/verification/vendor-refresh-2026-11-13.md. No Sonar configuration.

Final independent review confirmed source/patch correctness and evidence accuracy, inspected all four captures, and found no actionable issues. Research TASK-55/56 remain To Do. Integration is ready for the requested separate vendor commit and PR publication on codex/vendor-refresh-hook-research.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Refreshed Rune from bba30a61 to fetched dev tip c4eda058; CallAtlas and java-callgraph2 already matched upstream tips. Rebased/checksum-pinned all four Rune patch alternatives, preserved observers and shader guards, enabled release tooling and upstream patched crate overrides, and corrected added-file migration coverage. Full verification passed 27 stages with 784 release tests and no ignored tests; focused GPU and optional compiler freshness journeys passed. Independent review and Code Health safeguards passed. Evidence includes packaged dark/light before/after captures; upstream warnings, intentional runtime shader fallback and existing filter/card overlap are documented.
<!-- SECTION:FINAL_SUMMARY:END -->
