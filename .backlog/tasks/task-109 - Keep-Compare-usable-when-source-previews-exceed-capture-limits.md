---
id: TASK-109
title: Keep Compare usable when source previews exceed capture limits
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24T08:14:00Z'
updated_date: '2026-11-24T08:44:00Z'
labels: []
dependencies: []
type: bug
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compare aborts the Git outline for Base versus Worktree when generated binary files exhaust the source capture budget. Recurring Billing reproducibly crossed the 64 MiB text budget while reading untracked .cqc JARs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Oversized and binary paths remain visible and explain unavailable previews while small tracked edits still open.
- [ ] #2 Git ignore rules and tracked dot-directory changes are preserved.
- [x] #3 Paged results and previews retain comparison binding and verified source attribution.
- [ ] #4 CLI, MCP, and desktop expose the bounded capture behavior and the public query contract documents it.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reproduce the failure and separate discovery from content capture.
2. Preflight saved blobs and make content limits per-file availability outcomes while retaining deterministic Worktree binding.
3. Expose readable preview reasons and document the query behavior.
4. Verify real Git, CLI, MCP, desktop, and repository quality gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reproduced on the exact Recurring Billing Base versus Worktree pair and in a standalone temporary Git checkout with five sparse 15 MiB binary files plus a tracked Java edit. The original CLI returned diff_too_large even with a one-row page. The renderer and MCP boundary regression are being handled independently of the shared core capture.

Separated Git path discovery, bounded content capture, and comparison assembly. The initial development selector passed all six stages. Independent review caught and verified a fix for tracked binary equality and mode-only changes. Individual CodeScene reviews scored 10.0 on all eight touched Rust files with no findings. The development selector was restarted against the revised Rust files.

User requested closure and a local commit before further verification, which they will perform. The first final selector passed 28 stages, including Cargo check, Clippy and the new desktop preview renderer test. Two existing automatic refresh UI journeys timed out, leaving 31 stages unrun. Evidence: target/verification/task-109-final. First final-run timing in seconds: compilation and checks 320.97, setup 9.73, tests 401.67, cleanup 5.22, other 0.76. The retry was gracefully interrupted at the user request during integration compilation. Evidence: target/verification/task-109-final-retry. The real-Git capture regressions, MCP acceptance, native Compare gates, and performance checks remain pending. AC 1, 2, and 4 remain unchecked.

The staged CodeScene pre-commit safeguard passed on all eight eligible Rust files with no findings.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compare keeps Git change rows visible when binary, oversized, or budget-skipped content cannot be previewed. The revised development selector passed all six stages. The freshly built CLI returned 106 rows over three bound pages and opened a Java diff for the exact Recurring Billing Base and current Worktree. The original large JARs were absent from that replay. See implementation notes for deferred verification.
<!-- SECTION:FINAL_SUMMARY:END -->
