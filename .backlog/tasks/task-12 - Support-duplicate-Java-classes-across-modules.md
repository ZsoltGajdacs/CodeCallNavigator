---
id: TASK-12
title: Support duplicate Java classes across modules
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T07:19:00Z'
updated_date: '2026-11-09T08:01:00Z'
labels: []
dependencies: []
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Recurring-billing compiles but graph normalization rejects generated duplicate classes. Support differing definitions using source-set compile visibility and preserve ownership in a combined graph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Identical and differing definitions retain ownership and caller-context targets
- [x] #2 Maven and Gradle compile classpaths govern precedence and shadow diagnostics
- [x] #3 Real build acceptance, persistence, failure preservation and recurring-billing verification pass
- [x] #4 Typed metadata and separated context execution and normalization pass CodeScene checks
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extract typed metadata and engine/context/normalization boundaries. 2. Export stable source-set identities and ordered evaluated compile classpaths. 3. Inventory compile entries for class-name visibility, but stage and analyze only winning project source-set definitions; retain original module/source provenance and import only caller-owned outgoing edges. 4. Preserve strict engine completeness without retries or reduced precision; bound successful context scratch usage and suppress repetitive exclusion logs. 5. Verify real Maven/Gradle duplicate, shadowing, module-only, persistence, failure preservation and packaged-resource cases; replay actual recurring-billing compiled outputs; review CodeScene and package the final app.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created approved task. Planner and explorer confirmed context-local normalization, typed provenance, initial-versus-final Maven resolution, and unsafe upstream duplicate provenance. Extracted normalization and typed metadata; focused existing tests pass. Running real bytecode checks before context changes.

Typed extraction verified against real bytecode. Existing 8 Java build acceptance tests, duplicate Maven/Gradle precedence+SQLite tests, 30 Python tests, packaged-resource tests and Clippy passed. Reviewer findings addressed: absent directory fingerprints, explicit ordered-context metadata, pure Gradle artifact aliases, and exploded multi-release filtering. CodeScene complexity findings removed through focused helper extraction; production Rust and new tests score 10.0. Full recurring-billing build running; final external-shadow and methodless-context regressions in progress. Formatter-only edits to vendored Rune restored by comparison with pristine formatted sources.

Full Maven recurring-billing build succeeded. Disk exhaustion during engine exclusion logging interrupted validation; recovered 3.3GB from an inactive rebuildable incremental cache, restored all verified formatter-only vendor edits, and restored two verified frontend generation changes in recurring-billing. Replay uses successful compiled metadata to avoid repeating the frontend build. Signed local packaging succeeded before final retry addition; will rebuild final artifact.

Final review fixed module-info-only source-set eligibility and shared entry-point finalization. Final duplicate fixtures including external winners and mixed unconsumed Gradle archives pass; existing eight Java acceptance checks pass; packaged-resource tests pass; 31 Python tests pass; Clippy passes. CodeScene source scores 10.0 and safeguard passed (37 eligible files checked). Recurring compiled-output replay has reached context 11/13, exercising bounded retry. User decision requested on reduced-precision policy per review; task remains In Progress pending that decision and final verification. Synthetic probes did not reliably force the engine limit; actual recurring replay is automatic-retry evidence.

User clarified that only actual project source-file relations matter, not full dependency context. Removed reduced-precision retry and dependency bytecode from engine inputs; external archives remain visibility/shadow inventory only. Prior full-dependency replay completed 13 contexts with 26790 declarations and 53978 relationships, retaining ten generated classes in each module, but is superseded by source-focused verification now. No policy approval remains needed because strict failure behavior is retained.

Correction to earlier investigation notes: the completed full-dependency graph contained zero reduced-precision diagnostics, so the positive automatic-retry claim was not established. User clarified source-file scope; all retry code and precision downgrade policy have now been removed. Final engine inputs contain project winners only. External class inventories retain shadow correctness without dependency call-graph analysis. Source-only recurring replay and final acceptance are running.

Final source-focused recurring-billing replay PASSED across 13 contexts in 125.96s: 20581 declarations and 50014 relationships. Every node has a source location within recurring-billing; impl and test-utils each retain all ten generated message/notice classes (170 declarations each). No precision downgrades or retries. Final Maven/Gradle duplicate acceptance, existing eight Java tests, two packaged-resource checks, 32 Python tests, 99 default Rust tests and two real bytecode checks passed. Clippy passed. All scored source files CodeScene10.0; safeguard passed for37 eligible files. Gradle init script unsupported by CodeScene (attempted, recorded); SonarMCP intentionally inactive. Final independent review found no blockers. Evidence: evidence/analyzers/java-duplicates.md. No commits or pushes created.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented source-focused Java duplicate handling with typed metadata, shared classpath winner staging, qualified source-set identities and caller-owned graph composition. External archives affect visibility only and are excluded from engine inputs. Strict completeness remains; no retry or precision downgrade. Verified recurring-billing: 20581 declarations, 50014 relations, all source locations within the project; both duplicate generated class sets retained. Maven/Gradle acceptance, persistence/failure preservation, packaged-resource checks, 99 Rust tests, 32 Python tests, Clippy and signed packaging passed. CodeScene10.0 and safeguard passed; init script language unsupported by CodeScene, SonarMCP unconfigured. Evidence and chronological action log are recorded. Changes remain uncommitted; nothing pushed.
<!-- SECTION:FINAL_SUMMARY:END -->
