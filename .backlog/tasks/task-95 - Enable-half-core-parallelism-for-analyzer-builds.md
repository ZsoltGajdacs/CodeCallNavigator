---
id: TASK-95
title: Enable half-core parallelism for analyzer builds
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-19T06:00:00Z'
updated_date: '2026-11-19T06:48:00Z'
labels: []
dependencies: []
type: enhancement
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Compiler-backed refreshes can use native build-tool parallelism to speed up independent work. This configuration-only change covers production Maven, Gradle, .NET restore, and CallAtlas invocations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Maven and Gradle compile with a target of half the available logical CPUs and at least one worker. Gradle exports metadata successfully after parallel compilation.
- [x] #2 The .NET restore and CallAtlas subprocesses receive the half-core CPU target through their runtime configuration.
- [x] #3 Engine guidance requires safe parallel execution of heavy work and reuse of native scheduling.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Set Maven native -T 0.5C and Gradle --parallel --max-workers for analyzer compilation.
2. Keep Gradle metadata export in the same invocation. Run it after tasks finish under Gradle’s build-wide project-state lock and before the compiler journal completes.
3. Share a half-core worker calculation between Gradle, .NET restore, and CallAtlas and document the engine policy.
4. Run selector-chosen correctness checks and review changed source with CodeScene. No speed measurements.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented compile-only Maven and Gradle worker settings, scoped .NET runtime CPU targets, and engine guidance. Independent static review and scoped formatting passed. CodeScene MCP scored all four modified Rust files 10.0 with no findings. The first selector run passed build, lint, and unit stages, then stopped at integration preflight because fixture environment variables were unset. Retrying the complete plan with the existing fixtures configured.

Configured selector run reached real integrations. Maven classpath and private multi-module analysis passed. Gradle duplicate-class fixture reproduced a project state lock error in the root metadata finalizer under --parallel. Split compilation and export into their existing Gradle entry points and repeat the required checks.

Review caught that a separate Gradle metadata invocation could evaluate different source and output settings based on task name or parallel mode. Stopped that verification run and confirmed owned cleanup. Keeping the existing task names and a single Gradle invocation. Cached Gradle distributions expose the build-wide lock used by the task-graph lifecycle; the post-task export will take that lock before journal finalization.

Final selector run target/verification/changes-h9md8vx9 passed 46 of 49 stages, including Gradle duplicate class and reactor fixtures, Maven reactor and private Recurring Billing, C# integration and headless acceptance, packaging, cleanup, and CodeScene. No stage failed. Three desktop windows were deferred because macOS console availability could not be established, so the selector exited 3 and the task remains In Progress. CodeScene scored the four Rust files 10.0. The Gradle script suffix is unsupported, and an equivalent Groovy copy returned no score. Independent source review and real Gradle runtime verification passed. Recorded stage times were 457.0s compilation, 22.1s setup, 945.5s test execution, and 55.9s cleanup. No speed measurements were taken.
<!-- SECTION:NOTES:END -->
