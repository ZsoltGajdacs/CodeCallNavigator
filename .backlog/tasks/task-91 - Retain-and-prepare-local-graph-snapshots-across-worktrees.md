---
id: TASK-91
title: Retain and prepare local graph snapshots across worktrees
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-18T22:01:00Z'
updated_date: '2026-11-19T21:08:00Z'
labels: []
dependencies: []
type: feature
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reuse locally analyzed revisions across linked worktrees without graph commits or infrastructure changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Eligible analyses retain verified bounded bundles in the approved user cache.
- [ ] #2 Explicit preparation reports progress and cleans up a detached worktree on success, failure and cancellation.
- [ ] #3 CLI, MCP and desktop expose equivalent lookup and preparation.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify bounded content-addressed storage and eviction using actual encoded graphs.
2. Bind eligibility to exact Git commit and checked analysis observations at shared publication.
3. Add detached-worktree preparation and common CLI protocol, then expose equivalent CLI, MCP and desktop controls.
4. Run linked-worktree and client acceptance plus required quality gates.

5. Remove Compare target controls and capture the main graph picker selection for both sides. Rebase exact build identities on the worker, negotiate additive engine support, and share the existing detached preparation lifecycle with CLI and MCP.

6. Verify Maven-only retention with TypeScript present, both Compare endpoints, historical target failures, and legacy engine compatibility through the deterministic selector and CodeScene.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented versioned user-local bundle retention, exact Git input checks, observed dependency identity, and detached preparation. CodeScene reviewed cache/storage and preparation code. First sandboxed verification could not read the benchmark profile; the outside-sandbox run exposed compile and Clippy issues that were fixed. Full selector rerunning.

TypeScript and Maven probes prepared, retained, reused, and cleaned up real detached graphs. The tracked-symlink regression probe published only a working graph. Verified store eviction and corruption recovery in the previous full run. Hardened generated-source ownership and skipped Git subprocesses for non-repository refreshes. Final selector run is changes-m3_yrcju.

Latest full run changes-m3_yrcju passed compilation, Clippy and unit checks. Rust integration stopped on three desktop automatic-refresh settle timeouts despite removing Git process launches for non-repositories. The latest compiler-backed tracked-link probe passed. The C# preparation probe could not certify owned descendant cleanup on this host. Worktree recovery confirmed inactivity and removed the probe checkout. Keep this task In Progress pending the full gate.

Compare now uses the main picker for both endpoints via capability-negotiated exact targets. In changes-w0ivo4yp all 67 executable non-CodeScene stages passed, including the Maven-only and desktop snapshot regressions, integration, packaging, and benchmarks. Three native visual gates were deferred because the console was locked. The run exposed a CodeScene complexity finding in capability parsing. The follow-up typed the optional snapshot capability and the focused CodeScene review and safeguard passed. Full final selector rerun follows.

Final candidate changes-mkec_53m passed 59 stages, including Clippy, unit and integration suites, package build, real Maven-only and desktop snapshot regressions, and C# desktop acceptance. Fixed the native Compare fixture so Git preserves intentional CRLF inputs. The fixture then reached native window setup, where AeroSpace did not register the requested owned window. The native Compare gate failed and 13 stages did not run. Ran the skipped CodeScene source review and safeguard separately on the final diff: all 17 sources scored 10.0 and the safeguard passed. The prior changes-w0ivo4yp run passed query and storage performance gates. TASK-91 stays In Progress pending full verification.
<!-- SECTION:NOTES:END -->
