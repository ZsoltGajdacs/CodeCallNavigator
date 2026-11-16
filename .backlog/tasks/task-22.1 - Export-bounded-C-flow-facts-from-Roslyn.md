---
id: TASK-22.1
title: Export bounded C# flow facts from Roslyn
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-16T07:11:00Z'
labels: []
dependencies:
  - TASK-21
references:
  - vendor/callatlas/src/CallAtlas.Core/CallGraphBuilder.cs
  - vendor/callatlas/src/CallAtlas.Core/CodeMap.cs
  - vendor/callatlas/src/CallAtlas.Core/CodeMapJson.cs
  - patches/README.md
  - analyzers.lock.json
documentation:
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
parent_task_id: TASK-22
type: feature
ordinal: 22010
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
C# analysis already holds Roslyn syntax, semantic models and assembly-qualified callable identities, but CallAtlas exports neither statement trees nor per-site flow targets. Add a versioned flow-facts capability to the existing extraction pass, without changing graph inclusion or native call binding. This is the raw exporter boundary; Rust ingestion and desktop enablement are separate children. The initial supported subset is synchronous structured bodies; unsupported evaluation/control constructs must become bounded opaque regions, not eager guessed calls. All upstream edits must be captured in the checked-in combined patch and bootstrap metadata.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Raw pinned-analyzer output has an explicit flow capability/version and deterministic per-callable facts for block/expression-bodied methods, explicit accessors, local functions and top-level bodies where the existing graph has an unambiguous identity.
- [x] #2 Supported straight-line statements, if/else, ordinary loops, simple switches, return/throw and short-circuit/conditional expressions retain evaluation order; unsupported syntax produces explicit partial/opaque facts without exposing nested calls as unconditional.
- [x] #3 Function and call-site spans map to exact original UTF-8 source bytes, including Unicode, CRLF and BOM inputs; call targets reuse compiler/graph identities and preserve declared targets, candidates and unresolved/reference distinctions.
- [x] #4 Export depth/node budgets fail visibly at the callable boundary; callbacks and local-function declarations do not become eager execution merely through lexical nesting.
- [x] #5 Real Roslyn fixtures cover overloads, duplicate type names across assemblies, same-line call sites and deferred bodies; enabling facts leaves existing nodes, edges, entries and unrelated diagnostics unchanged.
- [x] #6 The recorded patch, base revision, checksum and exact-previous-patch upgrade remain reproducible; stale/incomplete capability output is distinguishable and existing upstream failures are compared with a baseline.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pin raw exporter behavior with failing real-Roslyn tests for capability/JSON compatibility, stable per-site compiler targets, grouped top-level bodies, exact source spans, bounded unsupported/deferred structures and unchanged graph projection. Preserve the upstream 262-pass/29-failure baseline separately.
2. Add focused typed flow-contract, JSON, source-offset, statement/expression and per-site-binding modules. Reuse existing worker-local binding results and canonical BodySite identities, keeping aggregation/counts and graph inclusion unchanged.
3. Export a conservative synchronous baseline and detached/opaque unsupported bodies with explicit depth/node limits. Preserve UTF-8 bytes including BOM; no inferred synthetic lambda identities or aggregated-edge-to-site guesses.
4. Test Write/Read/Write, absent old capability, missing current fields and unknown versions; run focused Roslyn tests and compare upstream baseline failures and graph fixtures.
5. Review/refactor upstream source with independent reviewer and CodeScene; capture all source/tests in the combined checked-in patch, preserve the exact previous patch checkpoint and checksum, and verify clean/repeated/previous migration plus unknown-edit refusal.
6. Run full verification outside sandbox with validated TS/private Maven/profile inputs; record evidence and resolve regressions before completion or commit. Rust ingestion and desktop availability remain TASK-22.2 onward.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created feature/task-22-function-flowcharts; committed verified planning baseline as 6162d4f. Planner completed and explorer identified worker-local extraction binding seam. Verified original CallAtlas checkout matches recorded patch exactly. Live full prerequisite check passed using documented installed fixture/profile inputs; Sonar is inactive because .sonar/config.json is absent. Upstream baseline: 291 tests, 262 pass and 29 existing identity-expectation failures; preserved separately before production edits.

Tests-first checkpoints: implementer added 11 real-Roslyn FlowFactsTests and observed all fail on missing flowFactsVersion/flowInventory/rejection behavior (/private/tmp/task22-flow-red/task22-red.trx). Root added a real CallAtlas patch-migration suite sharing the existing Rune harness; its new pre_flow checkpoint test failed as expected before metadata existed. Corrected the harness to retain each patch format abbreviation (CallAtlas 7, Rune 8), preserved the exact old patch/checksum, then all 10 dependency patch tests passed. CodeScene review of tools/test_dependency_patches.py: 10.0, no findings. No upstream graph behavior changed by the root-side bootstrap preparation.

Independent reviewer verified pre-flow checkpoint bytes against committed prior patch and independently reran all 10 dependency tests successfully; no blocking findings. Applied suggested addCleanup restoration for bootstrap.ROOT and temporary directories on setup failure; reran all10 tests successfully and Code Health remains10.0. Tester found raw architecture baseline paths are absolute; exact recorded task-owned fixture root will be used after ownership/content checks, keeping all existing graph fields in parity comparison. An early parity attempt used an old CLI binary and was correctly rejected on absent flow capability; explicit CLI rebuild is required before acceptance.

Independent early production-CLI acceptance passed: all existing baseline fields equal (6nodes/6edges),6deterministic flows; restored real four-assembly collision fixture12flows/8same-name target sites retains ownership; actual BOM/Unicode/CRLF span exact; conditional/deep/wide opacity bounded. Report remains exploratory /private/tmp/task22-flow-early-cli-acceptance.json pending final rebuild. Review found user-defined truth/contextual conversion omission, wrong JSON version-type exception, and preallocation budget holes; implementer added regressions/fixes. CodeScene first draft FlowValidation7.36→10 after schema-family split and FlowSource9.24→10 after original-byte/UTF8 split. Exact pre-flow CodeScene comparison: CallGraphBuilder4.91→4.64 due new single-file threshold, requiring focused worker-state extraction; CodeMap9.38 and CodeMapJson7.81 stable. New FlowJson8.15 is being split before final review. No raw-export completion or final full gate pass claimed yet.

Source frozen after final review fixes. Complete upstream Release suite317executed/288pass/29fail with exact original failure names and0skips; all26new exporter cases pass. Explicit CLI Release rebuild0warnings0errors. Independent final reviewer approved; independent tester durable raw-cli-acceptance.json passes exact6node/6edge baseline parity,6repeat-equal flows, real4assembly/12flow/8same-name-site ownership and exact BOM/Unicode/CRLF+opaque limits. Captured complete156770byte upstream patch SHA35ac591914c03085029b53832fc2e8f917c7e8d4577ba6c1a108367787c61852; checkout matches recorded bytes. All10 real patch migration tests pass against refreshed patch including every historical checkpoint. Final CodeScene source results saved in code-health.json: all new modules9.38-10, builder4.91→5.20, existing CodeMap/JSON stable, Python helpers10. Required full project verification is starting; task remains In Progress pending its terminal result.

First required full run target/verification/full-0yeay2xu failed in existing freshness observer test setup before EOF assertion; later gates not run, cleanup completed. Isolated original test reproduced. Temporary instrumentation restored exactbytes: only512readablebytes of an incomplete initialJSON frame, childalive; EOF still exited0within2sec. Native sample confirmed stdoutwriter blocked in libcwrite whilemain continued polling. Independent kernelprobe proved macOS pipePOLLOUT reports writable despite EAGAIN at512bytes; initial kernelpipeprobe correction also failed and was replaced after actor/assumption audit. Corrected testfixture only to controlled blocking UnixStreamstdout (4KiBsendbuffer), child-onlywrites, observed output+alive+sustained nonwritability; existing actualpipeEOFtests and2sec deadline retained. All6freshnessintegration tests pass; independentreviewer approved; CodeScene9.68. No production observer edits. Full retry remains required before completion.

All 29 stages of full-sw04ibr8 passed, including release acceptance/benchmarks (27.8 minutes), storage, packaging/signatures, installed CLI/MCP, CodeScene and cleanup. Committed the independently verified observer test-only correction separately as 6071db0. A final semantic audit then found compile-time nameof operands were incorrectly traversed. Two new regressions failed before the fix and pass afterward: intrinsic stays atomic; real @nameof retains argument evaluation and reports a missing native binding targetlessly without graph changes. Final upstream suite is now 319 executed, 290 passed, exactly the same 29 baseline failures; all 28 added cases pass. CLI Release rebuild has zero warnings/errors; reviewer approved the delta. Refreshed complete patch to 158969 bytes, SHA173f6ea6a771f5d2dded30f68319c3b7aa8dd4384a61fa43ad2f0bd71b5f3a7f; all 10 migration tests pass. CodeScene changed upstream files remain stable at9.38/9.84. Final independent CLI probe and a fresh full run against this final patch remain required; the prior full pass predates this source correction and is not claimed as final-source proof.

Final independent raw CLI acceptance includes the new intrinsic/user-named-method probes and passes all prior parity, collision, byte-range and budget cases. Final full-38446p00 passed all 28 stages against patch173f6ea6a771f5d2dded30f68319c3b7aa8dd4384a61fa43ad2f0bd71b5f3a7f, including protected cleanup. All criteria are backed by raw-cli-acceptance.json, code-health.json and verification.json; prior failed/pre-intrinsic runs remain separate. No missing full gates or accepted waivers. PR analysis is conditional and no PR was opened; Sonar remains unconfigured/inactive.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added versioned, bounded compiler-derived C# flow facts with exact UTF-8 source attribution, native per-site targets, conservative opaque regions, strict snapshot validation and reproducible recorded upstream patches. All 28 new Roslyn cases and independent CLI parity/collision/source probes pass; the 29 pre-existing upstream failures are unchanged by exact name. Final full verification passed all 28 stages, independent review approved, and Code Health safeguards passed. Rust ingestion and native C# workflow acceptance follow in TASK-22.2 through TASK-22.4.
<!-- SECTION:FINAL_SUMMARY:END -->
