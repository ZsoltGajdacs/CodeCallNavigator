---
id: TASK-91
title: Retain and prepare local graph snapshots across worktrees
status: Done
assignee:
  - '@codex'
created_date: '2026-11-18T22:01:00Z'
updated_date: '2026-11-20T00:57:00Z'
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
- [x] #1 Eligible analyses retain verified bounded bundles in the approved user cache.
- [x] #2 Explicit preparation reports progress and cleans up a detached worktree on success, failure and cancellation.
- [x] #3 CLI, MCP and desktop expose equivalent lookup and preparation.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify bounded content-addressed storage and eviction using actual encoded graphs.
2. Bind eligibility to exact Git commit and checked analysis observations at shared publication.
3. Add detached-worktree preparation and common CLI protocol, then expose equivalent CLI, MCP and desktop controls.
4. Run linked-worktree and client acceptance plus required quality gates.

5. Remove Compare target controls and capture the main graph picker selection for both sides. Rebase exact build identities on the worker, negotiate additive engine support, and share the existing detached preparation lifecycle with CLI and MCP.

6. Verify Maven-only retention with TypeScript present, both Compare endpoints, historical target failures, and legacy engine compatibility through the deterministic selector and CodeScene.

7. Reproduce Maven snapshot failure when validate writes into target and generate-sources registers a later Java root.
8. Clean the app-owned detached checkout before builds and use checkout ownership for eligible generated Java roots. Preserve compiler, bytecode and Git verification.
9. Verify the fixture, private Recurring Billing revision, checkout cleanup, and CodeScene with the deterministic selector.

10. For an explicitly cleaned checkout, allow ignored regular files only in directory membership verification. Verify all consumed inputs against their existing Git and compiler rules.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented versioned user-local bundle retention, exact Git input checks, observed dependency identity, and detached preparation. CodeScene reviewed cache/storage and preparation code. First sandboxed verification could not read the benchmark profile; the outside-sandbox run exposed compile and Clippy issues that were fixed. Full selector rerunning.

TypeScript and Maven probes prepared, retained, reused, and cleaned up real detached graphs. The tracked-symlink regression probe published only a working graph. Verified store eviction and corruption recovery in the previous full run. Hardened generated-source ownership and skipped Git subprocesses for non-repository refreshes. Final selector run is changes-m3_yrcju.

Latest full run changes-m3_yrcju passed compilation, Clippy and unit checks. Rust integration stopped on three desktop automatic-refresh settle timeouts despite removing Git process launches for non-repositories. The latest compiler-backed tracked-link probe passed. The C# preparation probe could not certify owned descendant cleanup on this host. Worktree recovery confirmed inactivity and removed the probe checkout. Keep this task In Progress pending the full gate.

Compare now uses the main picker for both endpoints via capability-negotiated exact targets. In changes-w0ivo4yp all 67 executable non-CodeScene stages passed, including the Maven-only and desktop snapshot regressions, integration, packaging, and benchmarks. Three native visual gates were deferred because the console was locked. The run exposed a CodeScene complexity finding in capability parsing. The follow-up typed the optional snapshot capability and the focused CodeScene review and safeguard passed. Full final selector rerun follows.

Final candidate changes-mkec_53m passed 59 stages, including Clippy, unit and integration suites, package build, real Maven-only and desktop snapshot regressions, and C# desktop acceptance. Fixed the native Compare fixture so Git preserves intentional CRLF inputs. The fixture then reached native window setup, where AeroSpace did not register the requested owned window. The native Compare gate failed and 13 stages did not run. Ran the skipped CodeScene source review and safeguard separately on the final diff: all 17 sources scored 10.0 and the safeguard passed. The prior changes-w0ivo4yp run passed query and storage performance gates. TASK-91 stays In Progress pending full verification.

Approved plan based on the exact private revision, two removed temporary checkouts, and persistent preparation lease files. Checkstyle runs at validate and OpenAPI at generate-sources.

Reproduced against private Recurring Billing 3e975f1f3d1e with installed engine and MAVEN_OPTS=-DskipUi. Both Maven passes succeeded. Checkstyle ran in metadata, OpenAPI generated in compile, and only final metadata listed the OpenAPI root. The engine then rejected the snapshot with the original Java outputs diagnostic and removed its Git worktree.

The private retry passed Java output attribution and then failed Git directory verification. Maven logs and checkout membership confirmed asyncapi-generator writes an ignored asyncapi.yaml at the checkout root. User approved the additional scoped fix.

The rebuilt engine retained the exact private revision 3e975f1f3d1e with AsyncAPI generation enabled and the existing -DskipUi Java-only option. The installed CLI found the saved snapshot in the live graph library, compared it against both the same commit and WORKTREE, and the Git worktree list was unchanged after preparation. Evidence: target/verification/task-91-private/.

Final run changes-3fqxiz7l reached Maven snapshot acceptance. Positive retention and warm-worktree checks passed. The tracked-source rewrite failed verification as intended via its exact Git blob. Corrected the acceptance assertion to check the actual mismatch, then reran verification.

Run changes-5ntq_a5w confirmed the exact tracked blob rejection. Cleanup comparison had included HEAD, which the fixture intentionally advances between rejection cases. Compare registered checkout paths and still require removal of the temporary staging directories.

Final selector changes-k4rqov91 passed 62 of 65 stages with no failures. The Maven regression passed OpenAPI-like generation, ignored report retention, tracked rewrite rejection, ignored Java input rejection, and Git/on-disk cleanup. CLI/MCP, headless desktop Compare, packaging, query/storage performance and CodeScene passed. Native C# desktop, Flow Comparison and Flow Expansion were deferred because macOS was locked. TASK-91 stays In Progress. Stage times were 365.1s compilation, 99.4s package build, 20.9s setup, 976.2s test execution and 9.6s cleanup.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Prepare snapshot cleans its detached checkout before analysis and accepts eligible build-generated Java sources and ignored non-input reports while retaining compiler and Git verification. The rebuilt engine retained Recurring Billing revision 3e975f1f3d1e, and the installed CLI compared it against WORKTREE.

The final selector changes-k4rqov91 passed 62 stages with no failures, including real Maven regression, CLI/MCP parity, headless desktop Compare, packaging, performance and CodeScene. A separate cancellation probe verified Git unregistration and on-disk cleanup after SIGINT during Maven preparation. Evidence is under target/verification/task-91-private/.

The user explicitly accepted deferring the native C# desktop, Flow Comparison and Flow Expansion checks, which did not run because macOS was locked.
<!-- SECTION:FINAL_SUMMARY:END -->
