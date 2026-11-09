---
id: TASK-17
title: Speed up analysis while preserving results
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T13:07:00Z'
updated_date: '2026-11-09T13:49:00Z'
labels: []
dependencies: []
documentation:
  - evidence/performance/analysis.md
priority: high
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement approved performance plan: reduce repeated build startup, classpath processing, graph composition and UI/query work without changing analysis accuracy or adding persistent analysis caches. Preserve existing dirty lambda changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shared UI/CLI stage timings, optional CLI JSON report and resource/work counters are available.
- [x] #2 Gradle uses two invocations and Java artifact/source inventories are reused within the job without changing ownership, order or diagnostics.
- [x] #3 Composition, normalization and snapshots avoid redundant work while preserving validation and changing-input rejection.
- [x] #4 Bounded filter summaries, lightweight search policy reads and verified TypeScript worker preserve behavior.
- [x] #5 Full graph equivalence and real Gradle/Maven/compiler/cancellation regressions pass.
- [x] #6 Five release runs document timings/resources/work, target at least 15 percent median StoryEngine improvement and investigate regressions over 5 percent.
- [x] #7 Relevant Rust/Python/Node, packaging and CodeScene checks pass or remain explicitly pending.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Freeze baseline executable/adapters and capture five release runs per workload. 2. Add shared job timings/work counters, optional CLI JSON and complete persisted-graph comparison/resource harness. 3. Combine Gradle compile/final metadata, share full-analysis artifact indexes and parsed source inventories, buffer manifests and prepare at most two artifacts while retaining ordered serial selection/full validation/sequential bytecode engines. 4. Refactor typed composition, streaming normalization, class facts and snapshot traversal. 5. Add bounded all-filter summaries and scalar/legacy policy reads. 6. Retain one verified TypeScript worker for a single target and reprepare serially for multi/mixed selections. 7. Fix independent review findings, verify full-toolchain/focused tests, compare five release runs and record quality/package/resource evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created task and froze original release executable plus Java/C#/TypeScript adapters under /private/tmp/codegraph-perf-baseline. Captured five release baseline runs per workload. Planner/explorer confirmed immutable ownership and freshness boundaries. Implemented job-scoped metrics and CLI --timings, shared UI indexing/publication/initial-layout timings, complete persisted-graph equivalence harness and resource sampler. Java, TypeScript and supporting optimization slices delegated with distinct ownership. Focused TypeScript/compiler/process and graph/storage/snapshot/composition suites pass; changed source CodeScene reviews currently 10.0. Full Python/Clippy and Java acceptance now running. Existing dirty lambda work preserved; no commit or push.

Independent full-toolchain Rust suite passed 187 tests (two private recurring-billing cases require unavailable project metadata); 30 Node and 44 Python tests passed. Final shared-cache and completed-metrics focused regressions also pass. Reviewer findings fixed: TSV header error propagation, bounded TypeScript worker retention, complete normalized fingerprint comparison, and immutable completed timing reports. Artifact cache now spans the whole selected-build analysis, preserving independent contexts and verified external-to-project promotion. Clippy, packaging/dependency/signature checks, and CodeScene safeguard (60 eligible files) passed. Broad formatter side effect on vendor/rune was proven formatter-only against HEAD and fully restored before packaging. First measured optimization improved StoryEngine median 33.547s to 29.848s (11 percent), below target; continuing stage-guided optimization. Graph semantics match all workloads; only generated source hash volatility already present across baseline builds is explicitly reported by comparison harness.

Final stage-guided iteration buffered artifact manifests and introduced bounded two-artifact preparation with ordered serial selection. Re-ran 9 JVM context/cache tests, both real Gradle/Maven duplicate-class integrations, and the cross-build shared-cache regression; independent reviewer found no remaining issues. Final five-run release comparison on Apple M4 Pro/48 GiB: StoryEngine 33.547 -> 27.150 s (19.1% faster), Gradle 8.300 -> 5.942 s (28.4%), Maven 5.213 -> 5.085 s (2.5%), TypeScript 0.703 -> 0.440 s (37.3%); C# 2.660 -> 2.739 s (3.0% slower, below investigation threshold). All 25 runs succeeded and matched complete graph semantics plus freshness input coverage; runtime full hashes remain strict, while benchmark comparison explicitly reports only generated-file hash volatility already observed across baseline builds. Median sampled StoryEngine peak RSS rose 907.8 -> 961.8 MiB and max sampled temporary disk 56.5 -> 71.7 MiB; bounded job-local working data and measurement limits documented. Final 46 Python tests, Clippy, package/dependency/ad-hoc signature checks, and 61-file CodeScene safeguard passed. No persistent cache, source-set/engine reductions, commit or push. Detailed action/validation evidence: evidence/performance/analysis.md, analysis-summary.json, analysis-tests.json, analysis-code-health.json and raw baseline/optimized JSON.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the approved performance plan with shared timings, two-invocation Gradle analysis, job-wide artifact/source reuse, bounded ordered artifact preparation, streaming/typed graph composition, bounded filter summaries, lightweight persisted-policy reads and a verified single-worker TypeScript lifecycle. StoryEngine median analysis improved 19.1%, Gradle 28.4%, TypeScript 37.3%; Maven improved 2.5% and C# varied +3.0%. All 25 release outputs matched; relevant tests/Clippy/packaging/CodeScene passed. Two private recurring-billing checks remain unavailable, .gradle is unsupported by CodeScene and Sonar is unconfigured. Resource costs and generated timestamp comparison policy are documented. Existing dirty lambda work preserved; changes remain uncommitted.
<!-- SECTION:FINAL_SUMMARY:END -->
