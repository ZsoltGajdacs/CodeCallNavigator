---
id: TASK-1
title: Validate CallAtlas and JDT analyzer integrations
status: Done
assignee:
  - '@codex'
created_date: '2026-11-07T22:00:00Z'
updated_date: '2026-11-07T22:43:00Z'
labels: []
dependencies: []
type: spike
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Establish compiler-backed extraction on PacificFate and StoryEngine before building the native CodeGraph UI. No custom language analysis or heuristic fallback is permitted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CallAtlas extracts fixture and PacificFate project graphs with cross-project calls and explicit diagnostics
- [x] #2 An existing Java analyzer extracts Java21/25 fixtures and StoryEngine Gradle graphs including every compiled declaration and disconnected methods
- [x] #3 Record checked call chains, callback and DI limitations, pinned dependencies and any blocking failures
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Pin public analyzer sources and inventory toolchains. 2. Run fixture extraction. 3. Validate real projects without source edits. 4. Record acceptance evidence; stop UI work if either mandatory integration fails.

JDT failed real-project import despite successful Gradle build. Revise Java engine to java-callgraph2 4.1.0 with BCEL6.13, disable upstream method filters, preserve existing engine call types and unavailable-body references, verify declaration equality against BCEL metadata.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Pinned CallAtlas 916c2419c0cdb91b5b86b84df4842dadb17bb6b4; Release build passed. PacificFate solution scan passed: 19772 nodes, 71546 edges, zero unresolved and zero compiler/body diagnostics. Fixture initially degraded before restore; restored net8 reference assemblies and rerun passed (12 nodes/12 edges). JDT 1.63.0 archive SHA256 b1f2abe0b3cccb462cf64fb4a5587c7d09df60cf8be7748b152cbd89d018bdc4 downloaded from Eclipse. Java acceptance in progress.

JDT 1.63.0 fixture passes (15 declarations, 11 edges). StoryEngine real compilation via Gradle compileJava compileTestJava passes. JDT initial run failed buildStatus2/2356 errors; a fresh workspace with pinned Lombok1.18.48 still fails on Reactor generic inference. Per approved gate, native UI remains gated; evaluating existing bytecode engine java-callgraph2 with Java25-compatible BCEL, without custom semantic analysis.

Revised gate passes: CallAtlas C#; java-callgraph2 Java21 and25 fixtures13methods/21rawrelations each; StoryEngine13987methods/65014rawrelations. Engine success=true, frame limits0; all method_info declarations exactly equal BCEL metadata. Recorded777 project-input relationships involving undeclared inherited/reference endpoints, preserved as explicit unavailable-body references. Driver/metadata CodeHealth10, protocol/bootstrap17 tests pass. Evidence in evidence/analyzers.

Independent review found upstream CallAtlas dictionary keys omit assembly identity and can merge valid same-named declarations across solution projects; reopening acceptance. Implement a minimal identity/provenance serialization patch to existing engine (no parser or call-resolution changes), add real collision fixture and rerunC#gate. Java source normalization also needs authored lambdas excluded from generated-code filtering; cache pre/post snapshot and custom Gradle source-set fixes in progress. UI completion remains gated until corrected acceptance passes.

Corrected C# acceptance passes after checksum-pinned41-line identity serialization patch: compiler-bound IDs now include assembly provenance; local functions/top-level roots stay distinct. New four-project collision fixture26nodes/20edges passes, originalfixture12/12passes, PacificFate20494nodes/75296edges/0diagnostics/0unresolved (722nodes/3750relationships recovered versus unpatched upstream). Same evaluated AssemblyName across independent projects is explicitly rejected before extraction. No compiler binding or call-resolution behavior changed. Patch patches/callatlas-assembly-identity.patch SHA256 ec723795ee841b98c2efbf49a7f010cd3f22652889c0e5c34866461b0137c574, bootstrap verifies/applies it.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Validated existing compiler/bytecode engines against realprojects, Java21/25 fixtures and C# project-identity collision regression. Minimal C# serialization patch prevents silent merges; Java declaration completeness checked against compiled metadata. JDT rejected and documented; no custom semantic graph engine.
<!-- SECTION:FINAL_SUMMARY:END -->
