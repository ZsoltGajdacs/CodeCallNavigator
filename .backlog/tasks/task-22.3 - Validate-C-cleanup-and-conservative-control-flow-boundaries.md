---
id: TASK-22.3
title: Validate C# cleanup and conservative control-flow boundaries
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-16T07:11:00Z'
labels: []
dependencies:
  - TASK-22.2
references:
  - crates/codecallnavigator-core/src/flow/model.rs
  - crates/codecallnavigator-core/src/flow/builder/exceptions.rs
  - crates/codecallnavigator-core/src/flow/builder/loops.rs
  - tests/flow_core.rs
  - tests/java_flows.rs
documentation:
  - >-
    .backlog/docs/planning/doc-14 -
    Function-flowchart-investigation-and-delivery-history.md
  - >-
    .backlog/docs/doc-33 -
    Internal-documentation-index-and-migration-manifest.md
parent_task_id: TASK-22
type: feature
ordinal: 22030
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Existing shared cleanup machinery was built for Java/TypeScript. C# using statements/declarations, foreach disposal and lock release need correct scope-specific unwinding; catch filters run before stack unwinding, and iterator yield is not Java switch yield. Extend only the cross-language primitives required for a documented synchronous subset, with language extraction in the adapter. Complex unsupported cases must remain opaque/partial. Do not claim complete C# runtime semantics or invent async scheduling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Published support matrix distinguishes exact supported structure, conservative exceptional paths and opaque/unavailable constructs for try/catch/finally, using statements/declarations, foreach, lock, catch filters, guarded patterns, goto, await and iterator yield.
- [x] #2 Real .NET marker traces for nested using scopes, acquisition failure, early return, break/continue, throw/rethrow and throwing finally match chart paths; cleanup occurs only for acquired resources and on the correct exits, without impossible fallthrough.
- [x] #3 Synchronous supported foreach/lock cases preserve acquisition/evaluation and disposal/release order on normal and abrupt exits; unsupported custom/async variants are diagnosed instead of flattened into misleading ordinary actions.
- [x] #4 Filtered catches never appear as unconditional catch-all handlers and retain possible uncaught paths; unsupported guarded/goto transfers are visibly partial and do not fabricate exact destinations.
- [x] #5 Async/iterator bodies and deferred callbacks are not represented as certain eager execution; C# yield is never decoded using Java switch-completion semantics.
- [x] #6 Shared model changes preserve old artifact behavior or introduce explicit compatible version handling, with existing Java/TypeScript cleanup, switch and callback regressions passing.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recheck compiler APIs and current raw/core contracts. Freeze the preceding c937ed67 patch as pre_cleanup; baseline native failures remain29.
2. Add failing native/export and production CLI marker-trace regressions before lowering. Cover nested acquisitions, lexical using declarations, captured/null resources, completion replacement, loop exits, foreach failures and lock release. Preserve tests-first red evidence.
3. Extend raw C# Try/catch/resources/cleanup/finally contracts and validation using existing core version-1 primitives. Align nested unlocated spans deliberately if needed; do not change legacy Unsupported semantics or native graph binding.
4. Implement cohesive synchronous using/exception lowering. Acquire left-to-right, dispose reverse-only-acquired; capture expressions once and guard reference/nullable cleanup. Using declarations wrap the remaining lexical sequence with synthetic nesting counted against budgets.
5. Lower only compiler-proven synchronous foreach forms: evaluate collection/acquire enumerator once, protect MoveNext/Current/body, and apply proven disposal or guarded runtime IDisposable semantics. Reject uncertain conversions, ref/deconstruction, array/string indexing, custom ref-like and async variants as diagnosed opaque cases unless separately proven.
6. Lower ordinary-object Monitor locks with object capture once, lockTaken initialization, Enter inside try and guarded Exit in finally. Keep System.Threading.Lock and uncertain/dynamic routes opaque.
7. Keep filtered regions visibly partial using an opaque region plus synthetic normal-versus-exceptional completion outcome and propagation branch; add no fake call sites. Preserve possible outer cleanup/catch/uncaught routes. Await/iterator/guarded/goto/callback boundaries stay explicit and non-eager.
8. Compare real .NET runtime marker traces with chart paths and reject structurally impossible acquisition/cleanup/fallthrough paths. Check graph invariance, source/provenance, bounds/roundtrips, and existing Java/TypeScript cleanup/switch/callback regressions.
9. Publish the exact support matrix, independently review/refactor cohesive modules, run CodeScene/safeguard and full verification outside the sandbox with isolated profiling inputs. Finish only with all six AC and both DoD proven.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Forward review while TASK-22.2 verifies: no new shared IR is expected for the bounded synchronous subset. Extend only raw C# typed Try resources/catches/finally/cleanup serialization and validation; use existing Evaluate/Value targetless call sites for implicit operations so shared conservative exception paths include GetEnumerator, MoveNext, Current, Dispose and Monitor Enter/Exit without native graph mutations. Lower using declarations around the remaining lexical suffix, counting synthetic nesting toward budgets; capture using expression once; acquisitions left-to-right and reverse disposal only for acquired resources; guard reference/nullable cleanup. Ordinary-object lock evaluates object once, places Monitor.Enter(ref lockTaken) inside try and guarded Exit in finally; System.Threading.Lock remains opaque. Foreach acquisition precedes protected loop, MoveNext/Current are inside; distinguish known disposal, sealed non-disposable and runtime IDisposable guard, leaving unsupported arrays/string/ref/deconstruction/custom conversion variants opaque initially if not proven. Filtered catches must remain opaque before-unwind boundaries; await using/await foreach need explicit guards even without AwaitExpression. Real traces should cover failed second acquisition, null resources, Dispose throwing while outer cleanup still runs, return value before disposal, break/continue, GetEnumerator failure without dispose, MoveNext/Current failure with dispose, throwing finally replacement, and lock release visible to another thread. Reuse existing core cleanup tests and Java/TypeScript regressions; no generic framework or fabricated lowering graph targets.

Filtered-region refinement from actual core inspection: Unsupported currently returns only an unknown-labeled normal exit; it does not preserve exception escape by itself. Prefer existing-IR conservative completion encoding: Unsupported region, then a synthetic If whose label distinguishes exceptional versus normal OUTCOME, then a synthetic Throw propagation branch versus normal continuation. Use zero/unlocated spans, explicit partial diagnostics and no invented call sites. Do not phrase the condition as whether an exception is possible (a metaproperty that would make its false branch misleading). Verify both normal and escape paths, outer finally on both, possible outer catches, no eager filter calls, and unchanged legacy Unsupported behavior. A fresh core semantic field/version is unnecessary if this representation proves truthful in the tests.

Synthetic-span compatibility: core already recognizes nested 0..0 as the version-1 unlocated sentinel, while raw C# FlowValidation.CheckSpan currently requires nested ranges inside the callable. If synthetic completion/lowering nodes use 0..0, align raw validation deliberately and add callable-start-greater-than-zero regressions; retain rejection for malformed meaningful spans. Alternatively anchor derived scope operations to the owning region span with explicit synthetic labels. This is validation alignment, not a new completion contract.

TASK-22.2 completed and committed as 1e0465b after full-9dgrhuyk passed all41 stages. Beginning tests-first cleanup/control-boundary delivery. Preserve c937ed67 combined patch as checksum-pinned pre_cleanup checkpoint before upstream edits. Planner/reviewer forward audits identify existing IR-only lowering, synthetic completion outcomes for opaque filters, and explicit unsupported limits; recheck current compiler APIs and source before implementation.

Explorer current-state audit confirmed installed Roslyn public APIs and existing IR seams. Two additional mandatory regressions: top-level using declarations must protect subsequent grouped body sites through the same ordered lexical suffix processor, and every resource declarator must count toward synthetic depth because core protected(resources) recursively handles each resource. Explicit implicit null/HasValue guards must not resemble overloaded source != calls. Foreach requires resolved GetEnumerator/MoveNext/Current plus both identity CurrentConversion and ElementConversion initially; public using operations expose no DisposeMethod, so prove ordinary IDisposable conversion and leave dynamic/ref-like/pattern cases opaque.

Tests-first checkpoint observed: all6 FlowCleanupTests failed against the preceding exporter as expected (durable /private/tmp/task22-cleanup-red/task22-cleanup-red.trx). Existing core IR proof tests already pass2/2: synthetic normal/exceptional outcomes both unwind cleanup with no fake call sites, while legacy Unsupported remains normal-only. No shared core production change is needed. Pre-cleanup checkpoint migration suite passes12; checkpoint test source CodeHealth10. Beginning raw contracts and focused adapter lowering.

Lock proof refinement: verify the modern Enter(object, ref bool)/Exit(object) member shapes before emitting lockTaken lowering; legacy or ambiguous member contracts remain opaque. Primary compiler source: https://github.com/dotnet/roslyn/blob/main/src/Compilers/CSharp/Portable/Lowering/LocalRewriter/LocalRewriter_LockStatement.cs . Unsupported using-declaration outcomes must explicitly own/describe the remaining lexical continuation.

Independent incremental runtime acceptance: first real using scope red against frozen22.2, then green against new adapter; three resource completion scenarios now pass and seven forbidden traces reject (including failed acquisition disposing an unacquired resource, skipped outer cleanup, throw resuming normal). Reviewer async-boundary repro added3 observed red cases: top-level await-keyword bodies lacked whole-callable opacity and an uninvoked root async local declaration infected the outer callable. A cohesive suspension detector fix is under focused verification. Current native checkpoint344/315pass/29exactbaseline is pre-fix, not final-source proof.

Final provenance audit found actual wrong merged flows for sibling locals and generic-arity overloads under existing canonical graph IDs (evidence cleanup/provenance-red.json). Added compiler-owner uniqueness with existing resolved symbols, including bodyless methods/locals and a cached type member/accessor collision index; partial definitions normalize as one logical owner, identical sites deduplicate, only top-level may combine bodies. New body source selection uses body-bearing entries. Conservatively omit a body in a different file from the retained native declaration to preserve existing node/source contract. Real CLI regression first failed publication with the old mismatch, then valid cross-filepartial published with flow_unavailable after the fix. Native graph IDs/edges/diagnostics are preserved. Final native355:326pass,29exactbaseline;4ignoredC#CLItests pass. Refreshed patch218546bytes SHAb3464d3f2b01d465d551e1dffff8fe89d5b25ba921a84c8a584f20f157c1ad48; independent guard review approved.

First full run full-ez6cs1ud passed all runtime/cleanup-trace/analyzer/benchmark/package/signature/installed-client stages and cleanup, but terminal CodeScene failed because the top-level-only fixture returned score null. No waiver: moved its existing ProcessExit registration helper into that owning executable as a real static local function, preserving direct top-level using lifetime. Both affected files now10, independent review approves, exact failed quality command passes, and all32 packaged-engine cases pass again. Failed run preserved; full-bkd4h8bc now reruns all34stages outside sandbox with isolated profile. Production adapter/DLL/patch remains c36315a4... unchanged.

Final full-bkd4h8bc PASSED all34 stages outside sandbox with isolated profile, including32-case .NET trace gate, release integration/benchmarks, packaging/signature, installedCLI, independentMCP and CodeScene. Precommit safeguard passes18/18 eligiblefiles, noissues. Durable evidence: evidence/flowcharts/csharp/cleanup/{verification,full-cli-acceptance,code-health}.json. Historical native suite355/326pass/29unchanged baseline failures is reported separately; full integration itself is green. Sonar inactive: no project configuration. No PR requested, so no change-set PR analysis.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented bounded synchronous C# cleanup/control flow using existing shared IR: ordered using acquisition/disposal, compiler-proven foreach and Monitor lock, try/catch/finally and conservative filtered outcomes. Async/iterator/uncertain constructs remain visibly partial. Compiler-owner/source guards withhold ambiguous bodies without changing native IDs or relationships. Added real .NET trace acceptance and tests-first provenance regressions; support matrix documents limits. Full34-stage verification and CodeScene pass;355 native tests retain29 exact historical failures. Evidence and failed/corrected gate history are retained under evidence/flowcharts/csharp/cleanup.
<!-- SECTION:FINAL_SUMMARY:END -->
