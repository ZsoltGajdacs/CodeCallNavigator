---
id: TASK-36
title: Add project cache clearing CLI and desktop button
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-11T07:00:00Z'
updated_date: '2026-11-11T07:14:00Z'
labels: []
dependencies: []
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Users need to discard reusable CodeGraph inference data before manually starting fresh analysis, via the engine CLI and desktop.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI and machine protocol clear project inference cache under the project lock and preserve graph and unrelated files.
- [x] #2 Desktop Clear cache sits beside analysis, runs in background, prevents overlapping analysis, and reports results.
- [x] #3 Regression checks cover clearing, safety, protocol and next-analysis cache misses.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add shared validated deletion and engine CLI/protocol. 2. Connect desktop background job and adjacent button. 3. Verify real CLI, desktop behavior, cache misses, Code Health and workspace/package checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented shared validated cache deletion under AnalysisLock, clear-cache CLI and additive protocol requests. Added background desktop job and adjacent Clear cache button with busy guards and status/error feedback; preserved graph/navigation and documented behavior. Planner/explorer, implementer, reviewer and tester roles completed. CodeScene modified sources report 10.0/no findings (module wiring boundary.rs has no score). Focused core/CLI/protocol/UI tests and real TypeScript warm-clear-cold acceptance pass; cold result has zero hits and identical canonical graph. Full ignored workspace command failed in unchanged release-only performance tests and sandbox-dependent detached-process cleanup; no waiver requested and task remains In Progress pending full validation. Sonar inactive: no .sonar/config.json.

Final check and strict Clippy passed. Real desktop runtime cache tests: 3 passed; executable CLI/protocol tests: 4 passed; entrypoint tests: 5 passed. Packaging passed and signed dist/CodeGraph.app. Pre-commit CodeScene safeguard passed (16 eligible files, no regressions). Additional core/engine run hit an unchanged deadline marker test; immediate isolated retry passed. Detailed verification and full-suite limitations: evidence/verification/cache-clearing.md. No commit or push made.

Subsequent complete cargo test --locked -p codegraph-core -p codegraph-engine --all-targets rerun passed; deadline failure was transient. Full ignored desktop suite limitations remain.

User requested a local commit. Rechecked the complete diff and pre-commit CodeScene safeguard: passed, 16 eligible files, no regressions. Preparing the cache-clearing implementation, tests, documentation and verification record as one focused commit; no push requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented CLI/protocol-owned project inference cache clearing and adjacent async desktop Clear cache control. Verified preservation, busy guards, safety, retry, zero post-clear cache hits and identical graph output. Packaging, compiler checks, strict Clippy and Code Health pass. Full ignored suite remains unverified due unchanged release-only benchmark and sandbox process-cleanup failures; task stays In Progress.
<!-- SECTION:FINAL_SUMMARY:END -->
