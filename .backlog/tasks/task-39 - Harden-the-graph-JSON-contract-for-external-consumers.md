---
id: TASK-39
title: Harden the graph JSON contract for external consumers
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T08:37:00Z'
updated_date: '2026-11-11T08:59:00Z'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement approved CodeGraph-only contract hardening: typed portable graph, structure-aware canonicalization, minification, compatibility schema and guide, portable observed-input provenance and read-only verification. Preserve v1 semantics, identities and legacy readability; no CodeQualityChecker edits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Typed persisted model, structure-aware transforms and shared validation preserve graph semantics and opaque extensions.
- [x] #2 Versioned schema, consumer guide and compatibility tests cover old and new artifacts.
- [x] #3 Compact canonical bytes are generated once and match publication digest.
- [x] #4 Production analysis records portable provenance; read-only verify detects stale and unverifiable inputs with structured results.
- [x] #5 Independent consumer, real-language, freshness and publication regressions and required quality checks are recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Characterize current graph values, identity, ordering and publication. 2. Introduce explicit portable contracts, typed-known-field traversal and focused canonical ordering; share portable validation. 3. Centralize compact canonical bytes and exact-byte digest/publication. 4. Add optional versioned portable analysis input manifests and read-only verify CLI with match/stale/unverifiable results. 5. Publish full Draft 2020-12 schema and consumer compatibility guide plus independent JSON consumption tests. 6. Review/refactor, run focused and required Rust/compiler/package checks and CodeScene; record evidence and unresolved prerequisites. CodeGraph only; no identity redesign or CodeQualityChecker changes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created Draft DRAFT-1, promoted TASK-39, and began approved implementation. Initial artifact CodeScene review: 10.0. Sonar config absent. Delegated planner/explorer roles completed research, now implementing isolated schema and provenance areas.

Implemented approved plan including automatic minification and maintainability refactors. Detailed action log and evidence: evidence/graph-contract/README.md. Normal desktop/core/engine tests, strict Clippy, engine packaging, independent schema/consumer checks and real TypeScript/Java/C# checks passed. All modified source CodeScene reviews 10.0; safeguard passed. Full include-ignored workspace invocation remains failing on release-only performance tests, sandbox process cleanup and private CODEGRAPH_COMPILED_METADATA prerequisite; no waiver assumed. Keep In Progress, AC5 pending full-gate resolution/acceptance. No commit/push.

User explicitly accepted the documented test limitations and requested task completion and commit. Accepted exceptions: three debug-invoked canvas performance tests require release mode; sandbox detached-process cleanup test failure; unavailable private recurring billing CODEGRAPH_COMPILED_METADATA fixture. Real linked Java/C# tests passed with compiler cache/pipe access. Evidence remains evidence/graph-contract/README.md; these exceptions do not claim the full ignored suite passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Hardened the v1 graph JSON interface for independent external consumers: typed portable contracts, structure-aware canonicalization and validation, automatic minification with exact-byte digest/publication, schema and consumer guide, portable provenance and read-only verify CLI. Normal desktop/core/engine tests, strict Clippy, packaging, independent schema checks and real Java/C#/TypeScript acceptance passed. CodeScene reviews and safeguard passed. User accepted the documented full ignored-suite limitations.
<!-- SECTION:FINAL_SUMMARY:END -->
