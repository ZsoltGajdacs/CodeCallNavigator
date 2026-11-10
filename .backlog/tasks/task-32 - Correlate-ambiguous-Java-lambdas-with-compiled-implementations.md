---
id: TASK-32
title: Correlate ambiguous Java lambdas with compiled implementations
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T20:49:00Z'
updated_date: '2026-11-10T22:24:00Z'
labels: []
dependencies: []
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Java classfile line metadata cannot safely associate every lambda with source syntax, especially same-line, overlapping/nested or missing-debug-line cases. The current exporter uses syntax identities only for proven one-to-one mappings and warns when it retains native compiler identities, which can change after unrelated lambda insertions. The user explicitly deferred exact compiler/source correlation from TASK-31.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Same-line, nested and missing-line Java lambdas receive stable source-derived identities through verified compiled/source correlation without guessing by names or ordinal positions
- [x] #2 Insertion and reordering preserve unchanged anonymous identities; body edits and identical siblings remain correctly distinguished
- [x] #3 Compiler-bound graph and flow relationships remain unchanged, with ambiguity controls and real javac regression tests
- [x] #4 Documented fallback warnings are removed only for cases with proven correlation; analyzer and CodeScene checks pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add real-javac regressions for exact singleton and complete shared lambda correlation. 2. Separate source snapshots, lowered-site capture and BCEL symbolic verification; verify scratch outputs against actual classfiles using bounded debug modes. 3. Add v8 validated correlation records and refactor context-indexed Rust identity/flow binding; preserve native graph and singleton keys, assign tagged sorted complete group keys without fabricated group flows. 4. Integrate runtime exports, capability/provenance/cache and packaging; document diagnostics and limits. 5. Review focused changes, run Java/Rust/packaging tests, CodeScene reviews and safeguard; record evidence and finalize locally, no merge/push.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Approved clarifications: compiler-merged lambdas keep one native node with a grouped source-derived export identity; group member edits change that identity. Indistinguishable separate siblings use deterministic occurrences, without continuity guarantees across identical insertions/swaps. Local/anonymous type owners retain fallback. Refactoring is limited to the Java analyzer pipeline. Plan approved with supporting maintainability refactors explicit.

Regression-first evidence: Java v7 lacks lambda-correlation records; end-to-end same-line flow regression reached expected count failure (1 actual, 3 required). Initial Maven sandbox cache lock failure resolved using an offline process-local resolver lock wrapper. Source snapshot/lowering/verification and typed Rust correlation/binding implementations are proceeding in separate ownership areas; native graph enrichment remains separate from source flow emission.

Implementation complete: v8 source snapshots and memory-only javac lowering capture, shared BCEL executable-class verifier, strict typed correlation validation, context-indexed singleton/group identity binding separated from source flows, explicit fallback dispositions, versioned cache/provenance and packaging checks. Fixed the real -g:none absent-SourceFile case without changing native nodes. Independent review found no actionable findings. Verification: workspace tests and strict workspace all-target Clippy passed; 12 focused end-to-end Java tests, 20 broader public Java integration tests and 9 public real-JVM engine tests passed; complete Python suite 72 passed. Adapter build and final engine-only packaging passed. All 22 supported modified/new sources reviewed at CodeScene 10.0; pre-commit safeguard passed 22/22 eligible files. CodeScene does not support .kts (installation verification 5/5); actual Gradle build passed. No Sonar config; private recurring-billing fixture unavailable and skipped. Detailed evidence: evidence/analyzers/java-lambda-correlation.md. No merge or push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented verified compiled/source lambda correlation with maintainable source/capture/verifier boundaries and typed v8 bindings. Same-line, nested and missing-debug singleton lambdas get exact portable keys/flows; complete compiler-shared implementations get one grouped key without fabricated source flow. Native graph nodes/IDs/edges and singleton key bytes remain unchanged. All workspace tests, strict Clippy, 41 public Java acceptance checks, 72 Python tests, adapter build, packaged-resource tests and final packaged engine smoke passed. CodeScene reviewed 22 supported sources at 10.0 and safeguard passed. Build script .kts is unsupported by CodeScene but the real Gradle build passed; Sonar unconfigured; private fixture skipped. Evidence: evidence/analyzers/java-lambda-correlation.md. Completed locally on codex/standalone-graph-engine; no merge/push.
<!-- SECTION:FINAL_SUMMARY:END -->
