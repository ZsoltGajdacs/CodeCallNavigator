---
id: TASK-81
title: Observe generated Java inputs at compilation boundaries
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-18T10:51:00Z'
labels: []
dependencies:
  - TASK-80
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Pre-build snapshots reject legitimate generation and post-build baselines can hide concurrent authored edits. Slice 3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Maven and Gradle supported compile operations capture consumed input evidence
- [x] #2 Pre-compilation generation succeeds without self-triggering another build
- [x] #3 Concurrent edits and unsupported compiler behavior retain actionable consistency diagnostics
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add one shared JDK-only compiler observation helper in the existing metadata JAR. Maven observes standard compiler mojo executions and Gradle observes JavaCompile lifecycle callbacks, including native up-to-date/cache/no-source outcomes. Record a strict current-attempt/version contract, lexical source/classpath membership, content and required links, keeping owned class and annotation-generated outputs separate. Unsupported observations fail explicitly; no forced rebuilds, project edits or custom dependency scheduler.
2. Separate stable evaluated configuration guards from Java source consistency in all pre-build checkpoints. Validate native boundary observations again at final JVM export and in Rust before accepting preparation, translate only valid current-attempt consistency failures into exact-path InputsChanged, and retain final core fingerprints for session reuse/extraction. Preserve actual compiler failures and prior publication. Use existing native incrementality; no project-specific overrides or compatibility fallback.
3. Extend real Maven/Gradle fixtures for a fresh generated directory and generated Java rewrites, with native no-change/cache/no-source checks. Cover concurrent equal-metadata edits and a late rewrite of earlier compiled generated sources. Add a mixed Maven/TypeScript fixture proving generated JSON is read after producers even when TypeScript is selected first. Keep each fixture isolated for parallel execution and its mutation/recovery sequence serial. Use narrow pure tests for wire validation and unsupported states, not simulated successful compilation.
4. Require matching adapter capabilities and update bootstrap/package resource validation and the verification inventory together. Run the complete deterministic selector, Code Health review and pre-commit safeguard. Keep physical desktop and blocked later checks explicitly pending until final Slice 6 while the desktop is locked. Commit Slice 3 before continuing to shared engine sessions.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resumed after test-optimization checkpoint 04a9538. No further test optimization planned. Current source and native API research revalidated; all three existing broad guards must change (initial source_snapshot, authored_before, verify_compiled) to avoid rejecting producer updates before compile evidence is checked. Keep scoped_v1 unchanged and introduce a distinct compile-observation capability; no old-state compatibility. Hindsight search currently times out; approved scope and current code remain available.

Implementation draft complete and independently reviewed. Shared JDK-only journal, Maven native session-owned listener and Gradle JavaCompile lifecycle hooks preserve native actions and incremental/cache/no-source behavior. Required current-attempt/version evidence has before, consumedAfter (captured with consumer exclusions at the actual after boundary) and full after snapshots; comparisons never reproject old inputs through a changed filesystem. Rust verifies kind/SHA-256/membership/complete relevant link topology before bridging to existing core BLAKE3 fingerprints. Core link-binding observation was exposed without changing existing hash order. Initial explicit configuration guards are retained, reactor POM files guarded before evaluation, native configuration files added without advancing known baselines. Missing evidence cannot bless leftover classes. Only a normal reaped native exit may supply a typed consistency diagnostic; timeout, cancellation, spawn/I/O and signal failures retain original classification. Existing real deadline/cancellation tests assert this boundary. Matching resources are mandatory; no old envelope or observation fallback. Two pure wire tests and extended real fresh/native-cache/late-edit/generated-source/AP-alias/mixed-Maven-TypeScript fixtures await the selector. All modified Rust sources reviewed at Code Health 10.0 after cohesive guard/walker/task refactoring. JVM/helper/package source reviews green (one new collector 9.38); Gradle extension unsupported by CodeScene, installation checks pass and independent manual review completed. Added sha2 0.10.9 for JDK SHA-256 interoperability; Cargo lock update adds only its seven required packages. Full verification selected automatically by dependencies/resources/package changes. Desktop gates and blocked later checks remain pending for final Slice 6 by explicit user direction.

Verification checkpoints remain preserved under target/verification. Full6 changes-f6uohd_v passed all unit stages and the new real Maven/Gradle generated-input, annotation-processor alias, strict fresh/warm skipMain and selected-first mixed Maven/TypeScript regressions. Integration stopped at the existing process_protocol cancellation probe before its stalled wrapper started: its inert resources fixture lacked the newly required observer-JAR path. Updating only that narrow fault-probe prerequisite and early-exit diagnostics; cancellation, descendant, prior-artifact and timeout assertions remain intact. Full6 results: 41 passed, 1 failed, 32 not run; category/global cleanup passed with errors empty. No native-window checks reached. Earlier failed runs and their logs remain unchanged; integration Python startup regression and acceptance remain pending a complete rerun.

Full7 changes-9kbvi83t passed the corrected EOF/SIGTERM cancellation fault probe (0.56 s) and the new Java/mixed regressions again. A separate existing core process-ownership test failed because the worker published an empty PID file before writing its contents while the parent polled existence. Fixing the readiness handoff with same-directory atomic marker replacement, without changing ownership assertions, timeouts, production cleanup, or test selection. Full7: 42 passed, 1 failed, 32 not run; all automatic cleanup passed with errors empty and the owned runner reaped. Later integration Python/acceptance remain pending the complete rerun. This is a fixture race correction, not further verification scheduling optimization.

The interrupted Full8 selector left a running label during Rust integration. Its process and guard were absent on resume. The replacement run reclaimed 1,670,352,553 bytes from the marked inactive workspace through normal checkout-locked cleanup and preserved the interrupted report. Replacement changes-q97ej3qj: 58 passed, 1 intentionally interrupted before desktop, 17 not run. Static, unit, integration, C# flow, package/signature, packaged engine, independently installed MCP, Rust acceptance, and automatic refresh passed. Both process cases and standalone verification entrypoints passed. Category and global cleanup passed. Scope of the final locked-desktop deferral is recorded in TASK-84. CodeScene safeguard passed all 29 eligible files; the Gradle script was independently reviewed because the tool does not support that extension.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Maven and Gradle now observe generated inputs at native compiler boundaries. Real fresh-checkout fixtures verify normal generation, incremental/no-source/cache outcomes, generated aliases and Maven-to-TypeScript input ordering. Late edits and unsupported compiler behavior identify the actual input or operation and preserve prior graph bytes. Verification passed through automatic refresh with the accepted desktop and downstream gates pending at the final checkpoint.
<!-- SECTION:FINAL_SUMMARY:END -->
