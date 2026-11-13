---
id: TASK-58
title: Expose Git graph comparison through CLI queries and MCP
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T21:48:00Z'
updated_date: '2026-11-13T07:50:00Z'
labels: []
dependencies: []
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers need explicit graph differences against Git history without checking out revisions or ingesting complete graphs. Extend the legacy comparison foundation with full changes and snapshot-bound pagination; automatic refresh and UI are excluded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI query and MCP compare saved WORKTREE or committed endpoints with resolved identities, digests and correct freshness.
- [x] #2 Deterministic full graph and metadata changes preserve exact identities, uncertainty, ordered flow and opaque extensions.
- [x] #3 Bounded pagination is complete and rejects changed snapshots and oversized records explicitly.
- [x] #4 Legacy compare behavior and existing CLI/session/MCP contracts remain compatible after focused refactoring.
- [x] #5 Real Git and MCP acceptance, full verification, CodeScene and large-graph measurements are recorded.
- [x] #6 Comparison consumes owned snapshots for canonicalization, validates from borrowed JSON and groups borrowed entity references without cloning whole graph collections.
- [x] #7 Existing comparison records, ordering, uncertainty, opaque fields, generation binding, pagination and legacy compare behavior remain identical under regression and retained-input parity checks.
- [x] #8 Full verification and CodeScene pass, and repeated release RSS/latency measurements show reduced memory for both the original retained graph pair and an additional single-function flow change.
- [x] #9 Verification, analyzer bootstrap/packaging, fixture SDK selection and native dependency metadata support the latest installed .NET 11 SDK/runtime and Graphviz 16.1.1 as explicitly requested; no older task-local toolchain is used for final verification.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Preserve baseline CLI outputs and real compiler artifacts for exact parity and memory comparison. 2. Deserialize artifact validation from borrowed JSON and add an ownership-consuming comparison path for the CLI while preserving the existing borrowed API. 3. Replace owned grouping with borrowed references and materialize only emitted change values, retaining all existing grouped wire shapes and ordering. 4. Add regression coverage for duplicates, membership changes, flow changes, opaque extensions, missing/null and invalid artifacts; review and resolve CodeScene findings. 5. Remeasure release RSS and latency on both retained cases, run full verification outside sandbox, record final evidence and close TASK-58 on the new branch.

6. Per user correction, use the latest installed toolchains: update .NET 11 analyzer build/packaging and fixture SDK selection, align Graphviz 16.1.1 version/source/bottle checksums and CI/docs, test current-toolchain preflight, then rerun full verification with global current tools.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded approved initiative and plan. Added failing CLI comparison regressions before implementation (unknown query variant), then implemented portable snapshot retrieval, full typed differences, shared one-shot/session dispatch and thin MCP tool. Focused CLI and real stdio comparison acceptance pass. Review found graph.extensions metadata collision and partial-clone lazy fetch; fixed with explicit scope and no-fetch Git flags plus regressions. All 70 core tests and strict core/engine/MCP Clippy pass. Initial full preflight failed only for unset fixture/profile inputs; discovered existing paths and preparing rerun. Sonar is inactive: .sonar/config.json absent.

Independent reviewer confirmed fixes and found no remaining actionable issues. Independent tester added/passed two real-process Git-read cancellation/disconnect tests outside sandbox; CodeScene 10.0. All new/modified source files reviewed with CodeScene; findings resolved to 10.0. Pre-commit safeguard passed (26 eligible files at that point); final full CodeScene stage remains pending. Release benchmark on retained 113,628,316-byte compiler graph: 10,827 nodes, 17,331 edges; controlled label+diagnostic edits yield 2 records/7,492-byte response, median 4.3998 seconds and peak 4,017,881,088 bytes over three runs. Cost and stateless rereads documented in evidence/graph-comparison. Full rerun target/verification/task58-full passed preflight, formatting, bootstrap, compilation, MCP boundary and workspace Clippy; Rust suite running.

Final full run PASSED all 52 stages in target/verification/task58-full: 568 ordinary Rust tests, 661 release tests (zero ignored), 215 Python and 112 TypeScript tests, CPU/GPU and storage benchmarks, packaging/signature, installed CLI, independent MCP and agent journeys, CodeScene and cleanup. Private npm registry DNS warnings delayed builds but both private regressions passed; no waiver needed. All affected sources CodeScene 10.0; safeguard passed with 26 eligible files/no regressions. Added evidence/graph-comparison/README.md and latency-memory.json; source graph pair remains ignored under target/verification/task58-large-comparison. No UI changes or automatic refresh added.

Reopened at user request on 2026-11-13 for measured comparison memory improvements; user requested new branch codex/graph-comparison-memory. Prior investigation reproduced 3.72 GiB RSS and 4.87 GiB after adding one function diagnostic. Scratch owned-canonicalization and borrowed-validation prototype reached 2.07 GiB; production changes now also include borrowed grouping. Preserve .idea/ and all comparison contracts. Compact typed flow representation is outside this change.

Implemented ownership-consuming rich comparison, borrowed JSON validation (graph and provenance), and borrowed grouping with unchanged singleton/multiple wire shapes. Added four core regressions after a missing-owned-API failure; all 74 core tests, core strict Clippy and affected-source CodeScene (10.0) pass. Focused CLI comparison/query/session/public-command tests pass. Captured three immutable before samples per retained case and full envelopes: original maximum 4,014,800,896 bytes RSS / median 4.1666s; one additional flow diagnostic maximum 5,558,403,072 bytes RSS / median 4.8997s. External .gitignore edit adds /.idea/* and is being preserved/excluded from task commits.

Final implementation review is clear after fixing borrowed-wrapper extension copies and clone-before-validation. Core suite now 76 tests; all core strict Clippy/check and six source CodeScene reviews pass at 10.0. Independent tester proved full query envelopes/pages plus legacy stdout/stderr/exit-code equality across 12 real-Git cases (including invalid types/provenance/references), and all three focused release MCP comparison/cancellation/disconnect tests pass. Three before/after release samples per identical retained input: original peak 4,014,800,896 -> 2,223,652,864 bytes RSS (44.6% lower), median 4.1666 -> 2.9233s; flow-change peak 5,558,403,072 -> 2,226,896,896 bytes RSS (59.9% lower), median 4.8997 -> 2.9124s. Every measured full envelope equals its pre-change response. Full verification running under target/verification/task58-memory-full; evidence in memory-optimization.json and memory-parity.json.

Initial memory full run stopped at preflight after global tools had upgraded to .NET SDK11.0.100 and Graphviz16.1.1. The user explicitly instructed use the latest. Stopped preparing older local tools; final validation will use globally installed latest versions. Updating owned bootstrap/build overrides to net11.0 without editing upstream sources, fixture SDK11 selection, verification checks and Graphviz native locks/checksums. No compatibility gate is being bypassed.

Latest-toolchain integration exposed two failed variants of one premise: global MSBuild framework overrides could retarget both CallAtlas projects while their declared targets remained net10. Single TargetFramework override mismatched restore; dual TargetFramework/TargetFrameworks still failed because project-reference negotiation strips global target properties. There are two project owners (CLI and referenced Core); both must declare the actual net11 target. Stopped override work and selected the repository-standard checked-in upstream patch approach: refresh combined CallAtlas patch to target both projects at net11.0, preserve previous net10 patch/hash as a bootstrap checkpoint, keep upstream revision unchanged and use standard dotnet build. All upstream edits will be captured immediately in patch/lock metadata. Global Graphviz16.1.1 and SDK11 remain the final verification tools.

Latest-toolchain integration is complete with global SDK11.0.100/runtime11 and Graphviz16.1.1. CallAtlas CLI/Core/Tests target net11 via the recorded combined patch; prior net10 patch/hash preserved as pre_sdk checkpoint, upstream revision unchanged. Standard CallAtlas build and DLL --help pass; bootstrap14/package15 tests and engine compilation pass; current/pre_sdk hashes and exact canonical checkout patch verified. Project native graphviz/source/bottle locks align16.1.1 and official source checksum/cached OCI bottle checksum. Preflight/current-version tests and CI/source/package Python coverage pass (47 tests). Fixed stale net10 analyzer path in mediatr_type_identity acceptance; keep older target reference-pack fixtures. Supplemental upstream dotnet test builds all three projects but has 262 passed/29 failed legacy assertions (e.g. expected unqualified IDs while longstanding assembly patch intentionally emits qualified IDs); those assertions were not changed as unrelated to comparison/toolchain behavior. Required full runner now executing at target/verification/task58-memory-latest-full.

User clarified .NET10 compatibility is unnecessary and targeting11 directly is preferred if simpler, with10 only as fallback if11 cannot be supported. .NET11 already builds and runs successfully, so retain11 and remove the optional net10 MediatR/generated-test targets instead of adding compatibility machinery. Clean-stopped the in-progress full runner before editing those fixture inputs; it will restart from current final sources. Retarget MediatR fixture/generatednet10 cases to11 and regenerate locked NuGet assets, preservingpackagepins. Existing unrelated net8 multi-target fixture scenarios remain unchanged.

Final .NET11-only fixture simplification is verified: MediatR targetnet11 and all three NuGet lock files regenerated by force-evaluate restore; locked restore and fixture build succeed, MediatR12.4.1/contracts pins unchanged. Retargeted generatednet10 test inputs in C#policy/ASP.NET/linked-source to11 (unrelated net8 scenarios unchanged). All14 focused release compiler acceptance tests pass: ASP.NET6, C#policy4, MediatRdispatch1, identity2, linked-source1. Source CodeScene10 and diff checks pass. Prior latest full run was explicitly interrupted before fixture edits; it is not reported as passing (cleanup had unverified descendant status). Fresh final full run starts under target/verification/task58-memory-final-full with system .NET11/Graphviz16.1.1 and isolated saved-workspace profiling.

Clarification on the supplemental upstream suite: tool stdout reported 291 total / 262 passed / 29 failed after all three net11 projects built. Confirmed failures include longstanding unqualified-ID assertions (e.g. IShape::Draw and top-level Program) versus the assembly-qualified patch output, but stdout was truncated and no filesystem log retained, so not all29 causes are classified or proven pre-existing. This supplemental command is outside verify-full.sh; do not claim it passed or classify every failure. Retained successful current .NET11 bootstrap evidence is in the final full runner bootstrap log.

Final full attempt target/verification/task58-memory-final-full stopped at release acceptance: 666 passed/1 failed/0 ignored. Both private Maven regressions passed despite registry DNS retries. Failure was unsupported_handler_reports_compiler_error_instead_of_internal_exception at its dotnet restore success assertion, which did not include restore output. Investigating the actual restore failure before rerunning; later storage/package/installed CLI/MCP/CodeScene stages were not run. Full cleanup passed. Standalone Code Health safeguard passed for all17 eligible files. Additional current-release measurements remain ~2.07GiB with exact complete-envelope parity; concurrent-build timing samples retained separately.

Final full rerun PASSED all43 stages outside sandbox at target/verification/task58-memory-final-rerun: 574 ordinary Rust, all667 release tests with0 ignored, 218 Python and112 TypeScript, CPU/GPU/storage benchmarks, package/signature, installed CLI, independent MCP/agent journeys, CodeScene and cleanup. Exact direct restore reproduction succeeded; original failure cause remains unknown because prior output was omitted. Added only restore status/stdout/stderr to assertion, reviewed independently and CodeScene10; both identity tests passed in final full run. Final idle before/after three-sample measurements use verified packaged CLI: original maximum RSS4006789120 ->2227568640 bytes (44.4% lower), median4.6929 ->2.9113s; flow-change5560090624 ->2225913856 bytes (60.0% lower), median5.1066 ->2.9469s. Every full measured envelope equals immutable baseline. Updated evidence/graph-comparison/memory-optimization.json, memory-parity.json and README. Supplemental upstream TRX retained and independently classified: all29 observed first failure sites are unqualified-ID expectations/lookups,26 snapshot and3 delegate qualification; no distinct net11 assertion failure identified, later assertions remain masked, no net10 control run. Current upstream patch delta only retargets3 projects. Required gates all pass; optional upstream suite is separately reported as262 pass/29fail. Sonar inactive; unrelated user .gitignore edit remains excluded. Final work is on codex/graph-comparison-memory, no push authorized.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reduced rich comparison memory by consuming snapshot graphs, validating borrowed JSON and grouping borrowed entity references while preserving the public borrowed API and complete CLI/MCP/legacy behavior. Final idle peak RSS fell3.73->2.07GiB (44.4%) and5.18->2.07GiB (60.0%) on the retained113MB graph pairs; median times4.69->2.91s and5.11->2.95s. Exact response parity,12 independent real-Git cases and all43 full-verification stages passed. Integrated current .NET11 and Graphviz16.1.1 with recorded upstream patch/checkpoint, fixture locks, CI and packaging. Evidence and chronological action log retained; remaining expanded JSON memory and supplemental upstream29 identity-expectation failures are documented. Local branch codex/graph-comparison-memory; no push.
<!-- SECTION:FINAL_SUMMARY:END -->
