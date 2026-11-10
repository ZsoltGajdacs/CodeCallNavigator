---
id: TASK-31
title: Review standalone graph engine refactoring
status: Done
assignee:
  - codex
created_date: '2026-11-10T20:17:00Z'
updated_date: '2026-11-10T21:31:00Z'
labels: []
dependencies: []
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Review the complete refactoring against the preserved master baseline before local integration; implementation checks alone do not establish that cross-package and desktop contracts are correct.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Complete diff reviewed and confirmed defects fixed with regression evidence
- [x] #2 Applicable compiler, analyzer, packaging and CodeScene checks pass with baseline limitations recorded
- [x] #3 Reviewed fixes committed on the feature branch with clean working tree; local master unchanged and no merge or push
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Review master...feature by core/protocol/cache, TypeScript inference, Java/C# adapters/upstream patches and desktop/build integration. Reproduce findings before minimal fixes; independent review and focused regression tests for each unit. Run workspace, real-toolchain and packaging checks plus full-branch CodeScene gate. Commit reviewed fixes and evidence on codex/standalone-graph-engine. Recheck refs and working tree; do not merge or push, per the latest user instruction.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User steering: continue all verification and fixes but DO NOT MERGE at the end. Earlier merge authorization is withdrawn; preserve local master at its baseline.

Review findings confirmed with failing regressions: Java/C# canonical-hydrate-export identity drift; compiler project-name/checkout-name collision; TypeScript literal-path semantic identity rewriting and compiler-generated import-path normalization; unnamed default-class method instability; regex lexical-context anonymous fingerprint collision; structurally typed global Set replacement enabling false observer links; desktop engine exit with inherited protocol pipe causing unbounded wait; ASP.NET custom HTTP-method metadata and return-type endpoint metadata providers omitted from routing blockers. Fixes are being made within original contracts, with no merge per user steering. Initial whole-branch CodeScene gate passed233eligible files; no Sonar configuration present.

User accepted deferring exact ambiguous Java lambda correlation to follow-up TASK-32; current review fixes proven unique mappings and emits explicit native-ID stability warnings for ambiguous cases. Additional confirmed fixes: invalid nested flow/architecture versions rejected; quoted encoded Java context paths normalized; actual Gradle/nested TypeScript project ownership and named-root path metadata corrected; macOS source aliases canonicalized without changing bindings; computed/quoted TypeScript declaration names stabilized. Packaging regressions proved index flags/external diff could hide edits; private-index/raw-diff correction passes6package+7dependency tests and preserves all realvendor indexbytes. Full CallAtlas262pass/29fail matchesexactbaseline.

Review fixes verified with final workspace379passed, strict all-target Clippy, Python77, TypeScript104, real-language51 including successful isolated retry after one NuGet restore failure, final integration8 and desktop8. Packaged app+engine build; all7workloads repeat/reversed-target/relocation byte-identical. Pinned vendors match complete checked-in patches; pre-commit CodeScene48eligible files passed. Independent native preservation/performance comparisons finishing before handoff. Master remains fb54f5f789d94c6cbc5fe86bf20010edea815b33; no merge/push.

Committed reviewed fixes as 5db7709. Final seven-workload native comparison against frozen b839ba3 preserves native IDs, complete edges, entry/flow/architecture/boundary facts, diagnostics and freshness facts. Expected deltas: six TypeScript React signature qualifications and corresponding export keys, analyzer provenance and Java identity/folder-root metadata. All7 deterministic repeats/relocations/reversed-target checks passed. Final matched timing runs and post-commit whole-branch CodeScene gate pending.

Final verification complete. Native preservation all7 explained; desktop8 and deterministic7passed. Matched full-mode median runtime +1.67%TypeScript/+1.45%mixed, below5%; sampled mixedRSS+6.69% with overlapping broad ranges, limitations documented. Whole-branch CodeScene master gate passed242eligible of336changed files. Source fixes5db7709; final evidence/task completion committed separately. No Sonar config; exact29CallAtlas baseline failures unchanged. TASK32 tracks deferred ambiguous Java correlation. Masterfb54f5f789d94c6cbc5fe86bf20010edea815b33 unchanged; no merge or push.
<!-- SECTION:NOTES:END -->
