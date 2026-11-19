---
id: TASK-99
title: Unify CodeGraph storage in one local library
status: Done
assignee:
  - '@codex'
created_date: '2026-11-19T12:07:00Z'
updated_date: '2026-11-19T17:08:00Z'
labels: []
dependencies: []
type: feature
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace repository-local generated graphs and separate snapshots with one transactional user library while keeping the compressed manifest format.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Regular analysis and refresh save graphs centrally and register verified SHA variants without a second build.
- [ ] #2 Overview, Compare, CLI and MCP use the shared library with saved build and analyzer choices.
- [x] #3 Commit verification rejects mismatches and insufficient evidence while keeping successful working graphs available.
- [x] #4 Publication, rollback, eviction and repository cleanliness are verified.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Consolidate core catalog, paths, locks and publication. 2. Separate engine verification from persistence and save analyzer details. 3. Cut over consumers and integration, update docs, then run the deterministic gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented the central transactional graph library, SHA-addressed verified manifests, effective build/analyzer metadata, and desktop/CLI/MCP cutover. Development selector changes-0t5n7blx passed. Full selector runs exposed and informed fixes for a symlinked desktop fixture and an observer cache hit masking a missing manifest. Full verification of the revised implementation is running as changes-qj1licun. CodeScene reviewed affected files; the repository safeguard before the last focused fixes passed for all 144 eligible files.

Final verification run changes-8221m_z9 passed 199 of 203 stages, including automated refresh, scripted desktop integration, real TypeScript and Maven commit binding, CLI/MCP journeys, recovery, eviction, package signing, performance, and cleanup. The console was locked and the runner deferred the native Compare, C# navigation, and flow-expansion gates. Keep AC #2 open for native validation. The full run selected CodeScene 1.5.6 from PATH, which omitted a required score. The CodeScene 1.5.20 MCP verified the affected file at 10.0 and the repository safeguard passed for all 148 eligible files.

Replayed the exact recorded CodeScene stage with the explicit /opt/homebrew/bin/cs-mcp (1.5.20) executable. The stage passed, including source reviews and the repository safeguard. Evidence: target/verification/changes-8221m_z9/codescene-homebrew.log. Rerun full verification with this executable once a native desktop session is available.

The console later unlocked. The live macOS ioreg command emitted a top-level dictionary, which the desktop preflight had treated as unavailable. Updated the parser to accept both registry shapes and exercised both locked and ready states. Live preflight returned READY outside the sandbox, development verification changes-s8ynczu7 passed, and the new full run changes-ge37b8s0 uses the known-good CodeScene executable.

The user explicitly accepted the current verification state and directed committing and closing TASK-99. Native C# navigation, Compare, and flow expansion remain unverified. The latest full rerun was stopped at the accepted checkpoint. The scripted desktop, real analyzers, CLI/MCP, packaging, recovery, performance, and current CodeScene results stand independently.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Centralized working graphs and verified Git snapshots in one graph library. Regular analysis and refresh publish the same eligible bundle for Compare, preserving build and analyzer selections and failing closed on uncertain commit attribution. Closed with the user’s explicit acceptance of the outstanding native desktop checks.
<!-- SECTION:FINAL_SUMMARY:END -->
