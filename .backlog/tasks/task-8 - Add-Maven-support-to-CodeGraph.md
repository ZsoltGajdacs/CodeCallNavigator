---
id: TASK-8
title: Add Maven support to CodeGraph
status: Done
assignee:
  - '@codex'
created_date: '2026-11-08T21:32:00Z'
updated_date: '2026-11-08T22:05:00Z'
labels: []
dependencies: []
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add Maven Java builds and reactors through the existing bytecode engine while separating build-tool preparation from graph analysis.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Real Maven 3.9.x, newer Maven 3, and Maven 4 standalone/reactor builds retain cross-module calls, tests and generated-source ownership.
- [x] #2 Shared UI/CLI selection discovers Maven, accepts POM paths, prefers wrappers and rejects overlapping reactors before compilation.
- [x] #3 Refactored shared Java analysis preserves Gradle graph identities, ownership, entry points and classification.
- [x] #4 Input fingerprints, incomplete output rejection, failures and cancellation preserve the previous graph.
- [x] #5 Pinned extension ships through bootstrap/package flow; docs, automated checks and acceptance evidence cover the feature.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pin Gradle behavior. 2. Extract build adapters and shared preparation contract. 3. Add Maven metadata extension, staged snapshots and overlap preflight. 4. Integrate discovery, CLI/UI, fingerprints and packaging. 5. Verify real Maven3/4 and Gradle builds, failure paths, package and quality gates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted Gradle adapter and shared Java preparation/normalization contract; implemented Maven extension and adapter, discovery/default selection and CLI/UI integration. Pre-refactor Gradle fixture baseline passed. Installed Maven3.10 acceptance suite passed8/8. Fixed review findings for all-file declared resource inputs and line-based Maven config parsing. User approved adding CodeScene guidance to AGENTS.md. Disk limit resolved by deleting rebuildable Rust incremental caches; future runs disable incremental compilation.

Independent review found resource-directory contents and Maven line-based argument parsing gaps; fixed and covered by pure and real-build regressions. Added path+kind selection identities and backward-compatible selected_targets session field; legacy and explicit-empty selection UI regressions pass. Verified native extension-classpath precedence and preserve only the effective definition. Maven4 import creates .mvn/target/project-local-repo; reproduced failure, narrowed fingerprint handling to generated marker/cache, proved config edits remain detected. Maven3.9 required Java17 extension bytecode for old Sisu discovery; all version matrices now pass. Restored1171 vendored Rune files only after exact rustfmt(HEAD) verification (one needed2 passes); vendor clean, package provenance passes. First packaged CLI smoke omitted external fixture parent and failed correctly; complete temporary fixture tree succeeds. Final evidence: evidence/analyzers/maven.md. Full Rust96/96, Python27/27, Maven3.9.9+3.10+4rc7 each7/7, packaged resources2/2, real packaged CLI POM/folder both publish4 symbols/1 edge, signature verified. Clippy and CodeScene safeguard pass (32 eligible files). Sonar inactive: config absent. App/ZIP rebuilt. No commits or pushes.

User requested a local commit. Rechecked complete worktree scope and whitespace; CodeScene pre-commit safeguard passed for32 eligible files with no issues. Preparing the Maven implementation, fixtures, evidence and approved AGENTS guidance for one local commit; no push requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Maven single-module/reactor analysis with existing project configuration and shared Java bytecode normalization; extracted build adapters, evaluated ownership preflight, reliable input snapshots, discovery/selection/session support and bundled metadata extension. Added CodeScene guidance by user approval. All acceptance criteria verified; see evidence/analyzers/maven.md. Tested Maven3.9.9,3.10.0,4.0.0-rc-7; future versions need validation. Delivered locally; no push requested.
<!-- SECTION:FINAL_SUMMARY:END -->
