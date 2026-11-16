---
id: TASK-22.2
title: Persist C# flows and serve them through CLI queries
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-16T07:11:00Z'
labels: []
dependencies:
  - TASK-22.1
references:
  - crates/codecallnavigator-engine/src/analyzers/csharp.rs
  - crates/codecallnavigator-engine/src/analyzers/csharp/ownership.rs
  - crates/codecallnavigator-core/src/flow/lookup.rs
  - crates/codecallnavigator-core/src/flow/inventory.rs
  - crates/codecallnavigator-core/src/query/operations.rs
documentation:
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
parent_task_id: TASK-22
type: feature
ordinal: 22020
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Rust AtlasMap has no flow fields, normalize_atlas never fills GraphData.flow_inventory, and shared flow lookup explicitly permits only Java/TypeScript. Connect raw C# facts to the existing flow/source/query contract through the same build/assembly/path mapping as graph nodes. This delivers a headless C# chart without changing desktop presentation. Keep unsupported/ambiguous callables unavailable or partial with reasons; old artifacts remain usable for graph exploration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An actual C# project analyzed through the production CLI yields a generation-bound flow query with chart, exact cached source, stable callable identity and preserved target certainty/provenance.
- [x] #2 Build, assembly, project and linked-source ownership are consistent across nodes, flow functions, calls, canonical artifact serialization, composition/pruning and SQLite round trips; duplicate or unmapped references are rejected.
- [x] #3 Old saved graphs without C# flow metadata remain readable and return a refresh/unavailable explanation; missing or incompatible required fresh-analyzer capabilities reject publication and retain the previous generation.
- [x] #4 Real CLI tests verify overloads, linked files in distinct compile contexts, generated/body-unavailable declarations, Unicode source ranges, and source retrieval after current files change or disappear.
- [x] #5 Before/after graph-equivalence checks show no unrelated node, relationship, entry-point or existing Java/TypeScript-flow changes; opening a flow performs no analysis or compilation.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pin integration with failing production CLI and capability tests, then cover malformed facts, identity/ownership, exact source, legacy caches and persistence.
2. Normalize C# flow after native graph/framework facts and before pruning through the existing raw-ID to node-index map. Preserve target certainty/provenance; reject corrupt or duplicate references.
3. Add typed compiler sourceCodePage and sourceFactsVersion 1 facts to distinguish actual decoding contexts. Capture the preceding exact patch checkpoint, refresh the combined patch/checksum, and verify all checkpoint migrations. Cache original bytes once per file and validate encoding/content per assembly context; retain graph/source-only non-UTF-8 behavior and exact UTF-8/BOM snapshots.
4. Share core meaningful-range validation between adapter binding and flow queries, preserving nested 0..0 sentinels and the separate query-display size cap.
5. Advertise C# through shared capabilities/lookup and document bounded support, historical refresh guidance and unsupported constructs. Existing desktop consumes this capability; TASK-22.4 still owns native/package workflow acceptance.
6. Exercise actual CLI queries without dotnet after source changes/deletion; overloads, linked contexts, generated/bodyless declarations, faulted publication, artifact/SQLite/composition/pruning and native/canonical graph parity. Validate mixed encodings against facts captured unchanged from the same analysis, not configured code-page assumptions.
7. Refactor cohesive normalization/source boundaries, independently review source/tests/helpers, run CodeScene and the safeguard, and complete full verification outside the sandbox with isolated profiling inputs. Close only after proven acceptance criteria and DoD.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-22.1 independent review identified a mandatory ingestion contract case: raw flow spans include actual UTF-8 BOM, while current CallAtlas ArchitectureFileFacts.Content uses tree.GetText().ToString() and strips BOM; csharp/architecture.rs currently persists it unchanged. Preserve the raw exact-byte span contract and bind to exact original source content during ingestion (with verified equality to compiler text after BOM handling), or reject mismatch explicitly. Add production CLI/persistence BOM+Unicode+CRLF regression; otherwise existing source query validation/highlighting would be offset by3bytes. This is not fixed or waived by raw exporter acceptance.

Compatibility checkpoint for implementation: distinguish owned UTF-8 compiler/disk mismatch (reject unsafe publication) from valid source encodings that the new exact-byte flow exporter does not support. Existing C# graph/source facts can be compiler-decoded from UTF-16 or other accepted encodings; do not make the new flow feature fail an otherwise supported graph-only analysis merely because fs::read_to_string cannot decode original bytes. Prove baseline behavior with a real UTF-16 source fixture, preserve its graph behavior, and make flow explicitly unavailable/unsupported with a reason if exact UTF-8 spans cannot be bound. Keep this policy in the source-binding boundary, not by parsing human diagnostic strings; test unsupported/generated/unowned paths alongside BOM-bearing UTF-8 success.

Shared-consumer audit: core query capabilities currently list Java/TypeScript; advertising C# truthfully will automatically enable the existing production desktop inspector action through its CLI capability lookup. Do not add a duplicate UI language gate or withhold a valid capability to preserve the old timing assumption. TASK-22.4 remains the native/package acceptance milestone; parent C# delivery stays incomplete until it passes. Fixture capability mocks can remain legacy examples. MCP result validation is language-agnostic and needs no graph logic. Update the explicit C#-unsupported claim in docs/cli-queries.md when CLI support lands. Shared source validation must preserve version-1 nested 0..0 spans as unlocated/synthetic sentinels, validate meaningful ranges against function/snapshot UTF-8 bounds (not immediate-parent lexical nesting), and keep the 16 MiB source-display limit at query time rather than turning it into graph-publication failure. Existing Java/TypeScript synthetic cleanup/bind/deferred-region spans and fixtures are regression inputs.

TASK-22.1 completed and committed as a622c62 after final full-38446p00 passed all 28 stages. Planner and explorer completed read-only integration/compatibility audits; current plan incorporates exact source-byte/BOM binding, graph-only UTF-16 compatibility, meaningful-range validation with legacy sentinels and truthful shared capabilities. Beginning tests-first Rust integration; preserve existing native graph facts and the independently installed MCP boundary.

Source-boundary refinement: add typed nullable sourceCodePage compiler metadata and sourceFactsVersion 1 for fresh analysis, preserving historical artifacts. This avoids guessing an encoding or parsing diagnostics when valid UTF-8 bytes were compiled with another code page. Explicit non-UTF-8 contexts remain graph/source-only; UTF-8 contexts require exact original/compiler agreement. Cache physical bytes by canonical file and validate each compiler context separately. Preserve the task-22.1 combined patch as a checksum-pinned pre_source_encoding_patch checkpoint, then refresh the combined upstream patch and bootstrap metadata. This bounded upstream contract addition supersedes the earlier plan statement that no upstream changes were intended.

Independent mixed linked-file encoding probe exposed a test-assumption error, not a proven ingestion defect: shared MSBuildWorkspace loading reported actual SourceText code pages differing from configured project CodePage values (one preserved run A/B=28591 and C=65001 despite A/B configured65001,C28591). Validate availability against typed facts captured unchanged from the same engine analysis, since a separate native rerun can observe different shared document loading. Keep plain UTF-8 linked-context overload tests, standalone UTF-16/Latin1 graph compatibility, and synthetic per-context conflict regressions. Record configured versus actual facts and this observed upstream limitation in evidence; do not infer encoding policy from project configuration or diagnostics.

Final evidence: evidence/flowcharts/csharp/ingestion/verification.json and cli-acceptance.json. Independent CLI acceptance passed 35 real operations, with engine SHA bf0809d34ddabd7444957a27a92a6554041e32a79484c2dae4feb98dfe7229bf and combined patch SHA c937ed67e92cc45783ad57289e8bfc15a4d55e3548bfe173ee7eacbe12f3990d. Focused core108/engine65, C# CLI4, query7, session2 and patch-migration11 checks pass; native331 total302 pass29 exact historical failures, all12 new source tests pass. Full verification full-9dgrhuyk passed all41 stages (2137.9 seconds), including real release acceptance, isolated benchmarks, packaging/signatures, installed CLI and agent/MCP journeys, CodeScene and cleanup. Rust/helper Code Health10, upstream touched baseline scores stable/improved, independent reviews approved. Sonar unconfigured; no PR requested, so PR change-set review is not applicable. Documentation links/JSON/diff checked after prose-only updates. Initial Rust red was observed in tool output without a retained log; native source red has a retained TRX.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
C# flows now persist and query against exact analyzed source using existing graph identities and ownership. Typed compiler encoding facts preserve graph/source support for non-UTF-8 contexts while withholding unsafe flows; stale or corrupt fresh facts reject publication without replacing the prior generation. Cached queries require no compiler and remain valid after live source changes/removal.

Verified all five acceptance criteria with real CLI/compiler fixtures, generated/bodyless and linked-context cases, faulted refreshes, graph parity and artifact/SQLite/composition/pruning checks. Independent review and Code Health passed; full-9dgrhuyk passed all 41 stages. Upstream retains its 29 exact recorded baseline failures; no new native failures. Cleanup semantics and native C# workflow acceptance remain TASK-22.3/22.4.
<!-- SECTION:FINAL_SUMMARY:END -->
