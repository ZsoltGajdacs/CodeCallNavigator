---
id: DRAFT-1
title: Streamline deterministic verification using affected test boundaries
status: Draft
assignee: []
created_date: '2026-11-19T07:06:00Z'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up to TASK-72. TASK-95 selected 935 registrations and 10 gates for five runtime paths in C#, Java and shared analyzer configuration. The longest recorded attempt, target/verification/changes-h9md8vx9, spent 26m56s in stages and deferred three native journeys. Rust integration took 8m04s and compilation across the run took 7m37s. Recent retained-snapshot attempts selected full verification because crates/codecallnavigator-mcp/tests/support/mod.rs was unclassified. Its seven direct consumers are MCP stdio test targets. Shared core paths still legitimately select broad coverage and both performance gates.

Proposed implementation plan

1. Replay recent recorded change lists against the baseline selector, preserving their base/index/worktree inventory history where available. Record selected IDs, actual failure/defer outcomes, toolchains, build targets, packages and timings. Include analyzer parallelism, desktop freshness presentation, retained snapshots, MCP support-only, tooling-only and an unknown/full-trigger path. Keep runtime measurements separate from offline plan comparisons.
2. Add explicit, reviewed ownership for shared test helpers, starting with MCP stdio support, and retain all known dependent test targets. Audit apparent full fallbacks before adding any path rule. Keep full fallback for unknown ownership and existing full triggers for build dependencies, bootstrap, packaging, vendor and release changes. Replaying retained snapshots must retain the cross-component and performance coverage that still applies.
3. Pilot narrower selection on existing leaf adapter paths using exact inventory test/gate IDs or a small reviewed mapping to them. Keep owning component unit coverage and real compiler/integration checks for affected languages, plus shared build dispatch, failure/retry and at least one real consumer crossing each affected boundary. For TASK-95, retain Java Maven/Gradle, C# build/cleanup/ingestion, relevant desktop analyzer coverage and shared dispatch. Audit the TypeScript automatic-refresh, installed CLI and agent journeys, plus native flow comparison/expansion, before excluding each one. Verify fixture and shared-runtime dependencies, including language-independent retry/publication behavior. Keep aggregate coverage for shared orchestration, contracts, mixed changes without a complete map and ambiguous files. Review broad artifact/query performance rules separately against actual benchmark call paths.
4. Carry chosen IDs through plan reporting, requirements, compilation targets, test execution, gate setup, package selection and cleanup. Keep category isolation, real fixtures, bounded scheduling, native/performance exclusivity, CodeScene, and truthful incomplete reporting. Report exclusions with their matched rules. Scope package/installation work with the gates that require it. As a separate measured optimization, trial a single all-target Clippy pass in place of check plus Clippy only if coverage of build targets, lint and compile-error detection is equivalent. Preserve the MCP dependency-boundary check. Compare the combined duration because the prior cargo check warms Clippy.
5. Add selector and execution-equivalence cases for mixed diffs, rename/delete, helper changes, inventory changes, full triggers and unknown paths. Replay observed Gradle and real desktop-to-CLI failures against the relevant changed-path plans, and use representative reversible regression mutations to prove the retained checks fail. Run baseline and proposed plans on matched inputs and comparable warm/cold conditions, with the native desktop available where required. Record setup, compile, test and cleanup times, exact selected/omitted IDs and coverage rationale, and keep narrowed rules only where measured gains and the regression evidence justify them.

Acceptance criteria

- Test-helper edits select all verified consumers without unrelated full fallback, and unclassified production and existing mandatory full triggers still select full.
- A mapped leaf change retains real applicable compiler, wire and consumer boundary coverage. Shared runtime/contract changes retain broad coverage. Every omitted expensive check has a documented risk mapping to retained checks or evidence its exercised path is unaffected.
- Reported IDs, prerequisites, build/package setup and actual executed IDs agree. Missing prerequisites and native deferrals remain visible and incomplete.
- Recorded baseline/candidate results show a faster representative iteration and equivalent detection of the selected realistic regressions. Existing full/compatibility entrypoints and cleanup guarantees stay intact.
<!-- SECTION:DESCRIPTION:END -->
