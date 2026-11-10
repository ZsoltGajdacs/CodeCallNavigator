---
id: TASK-21
title: Add source-derived function flowcharts
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T19:28:00Z'
updated_date: '2026-11-09T21:09:00Z'
labels: []
dependencies: []
references:
  - /Users/zsolt.gajdacs/Projects/interactive-review/src/flow.ts
  - /Users/zsolt.gajdacs/Projects/interactive-review/shared/types.ts
  - /Users/zsolt.gajdacs/Projects/interactive-review/web/src/flowLayout.ts
documentation:
  - docs/function-flowcharts.md
  - evidence/flowcharts/README.md
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A method call graph does not explain branching within a method. Explore reuse of interactive-review flow.ts, shared flowchart types and flow layout for a separate source-derived detail view. Keep parsing, flow construction and rendering separate from the inter-method graph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Selected Java and TypeScript methods open source-linked flowcharts with branches, loops, switches, exception handling and returns.
- [x] #2 Call links use CodeGraph analyzer identities and preserve uncertainty and overload distinctions.
- [x] #3 Validation covers finally, early exits, callbacks and Reactor pipelines; unsupported constructs are identified.
- [x] #4 Document language coverage and limitations.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add a shared flow module with a versioned per-callable statement model, control-flow builder, call attribution and diagnostics. Use exact cached-source spans and graph symbol IDs; keep flow data separate from inter-method edges. Port the review app's useful statement vocabulary and Reactor descriptions, not its monolithic parser/builder or heuristic call resolver.
2. Extend the existing Java javac source-inventory pass and TypeScript compiler worker to export detached statement trees, callable spans and compiler-backed call-site attribution during normal refresh. Reuse their current compiler contexts and binding results; no additional build, Java tree-sitter runtime or TypeScript compiler installation. Normalize Java signatures and TS declarations through existing identity maps. Ambiguous lambda/source mappings and unresolved calls remain explicit. Preserve every existing graph node and edge.
3. Persist an optional versioned flow inventory with each immutable graph generation. Reuse the architecture source snapshot, with UTF-8 byte spans over its exact original content and explicit adapter conversion from UTF-16 positions. Collect IR during refresh, build/layout a selected chart lazily in background jobs, cache results by generation and symbol. Older caches require refresh only for flowcharts; C# and declarations without source bodies display clear unsupported/unavailable states.
4. Implement normal/return/throw/break/continue/yield completion records. Route every abrupt completion through relevant finally/resource cleanup; clone cleanup regions per continuation when necessary so paths cannot resume at an impossible destination. A finally return/throw overrides the pending completion. Handle branches, loops, switch fallthrough, short-circuit/ternary conditions and nested callbacks. Label conservative exception paths; never render deferred callbacks as ordinary sequential execution. Unsupported constructs use explicit diagnostics/opaque regions; never fabricate a complete flow.
5. Recognize compiler-confirmed reactor.core.publisher.Mono/Flux operations for defer, map, flatMap, filter, switchIfEmpty, onErrorResume, then, zip, when and collectList. Draw assembly separately from subscription/deferred callback regions; distinguish empty/error alternatives and possible concurrency. Other operators remain opaque stages with diagnostics, and lookalike application methods receive no Reactor semantics.
6. Add an Open flowchart action in the inspector for supported methods. Show a dedicated workspace with the selected method's flow beside its cached source; selecting a step highlights its source span. Reuse existing source highlighting/selection and camera behavior through focused refactorings. Give flow navigation its own history, camera and source scroll, preserving the exact call-graph state on return. Call chips retain graph target IDs, certainty and provenance; candidates are shown rather than guessed, and linked project methods can open their own supported flowcharts.
7. Add dedicated Graphviz flow geometry for decisions, labeled branches, loops, joins and cleanup paths; reuse process execution/decoding without changing call-graph layout policy. Keep compiler export, binding, pure flow construction, layout, source rendering and workspace jobs separate. Cancel obsolete jobs and reject results for a different generation/symbol. Bound a chart to 1500 steps and 6000 links, with explicit unavailable/too-large state instead of silently truncating paths.
8. Verify pure flow semantics and real Java/TS fixtures, including finally on every completion kind, finally overriding a return, nested catches/resources, labeled loops, switch/yield, callbacks/async, same-line calls, overloads, interface dispatch, duplicate module identities, Unicode/CRLF and ambiguous lambdas. Compare executable fixture traces with chart paths; test known Reactor operators against a pinned real Reactor dependency. Prove node/edge identity invariance and persisted/old-cache behavior. Exercise native UI actions, source highlighting, target navigation/back, stale files, cancellation and minimum-window layouts; capture light/dark screenshots with production geometry.
9. Measure refresh time and cache growth on existing representative fixtures, run applicable Rust/Node/Python/analyzer and package checks, and review/refactor all touched source with CodeScene plus the pre-commit safeguard. Use package-scoped formatting and preserve the existing modified Rune checkout; isolate clean pinned dependency sources if packaging provenance requires it. Document supported semantics, incomplete/unsupported cases and the source-generation distinction. No C# flowcharts, AI explanations, PR diff views or recursive call inlining in this task.
Planning defaults: dedicated source-and-flow workspace and scoped recognized Reactor diagrams. TASK-21 remains To Do until implementation is requested.

Deferred capabilities are now explicitly tracked by TASK-22 (depends on TASK-21): C# flowcharts, optional AI explanations, PR before/after flow comparisons and bounded recursive call inlining. TASK-21 delivers the Java/TypeScript foundation only; TASK-22 remains separate future work.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation authorized with dedicated source-and-flow workspace and recognized Mono/Flux deferred-stage diagrams. Beginning shared IR/compiler exporters before UI integration; TASK-22 remains separate future work.

Implemented separate shared flow IR, compiler export and completion-aware construction. Actual Java and TypeScript traces now validate normal/return/throw/catch/finally, labeled loops and resource acquisition/cleanup. Existing node/edge snapshots compare equal before and after export. UI interaction testing found and fixed a fast-load initial-fit race and clipped call-target controls caused by dynamic footer geometry; method following and Back preserve call-graph navigation. Reviewer identified source-depth bounds and effectful switch-case ordering; guards are implemented and switch dispatch is being completed. Refactored reusable source rows and Graphviz execution without modifying Rune.

Completed compiler exports, exact UTF-8 snapshot binding, completion-preserving flow construction and lazy Graphviz workspace. Review fixes include ordered effectful switch tests, bounded deep source, shared Graphviz decoding and per-visit source-scroll ownership verified through A → B → A. Final verification: 250 Rust tests across regular/toolchain runs, 49 Python and 45 Node tests; all-target Clippy and package formatting; CodeScene source reviews and safeguard passed (62 eligible files). Packaged app passed strict codesign and restricted-PATH rendering using an isolated clean pinned Rune source; the existing modified Rune checkout was preserved. Two existing private-project checks lack their environment-provided fixtures and were explicitly excluded. Evidence and supported semantic limits are documented.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added source-derived Java/TypeScript function flowcharts beside cached source, with compiler-attributed call navigation, independent history/camera/scroll, finally/resource cleanup, deferred callbacks and recognized Reactor stages. Old caches remain usable and request refresh for flow data. Verified real executable traces, identity invariance, native UI interactions, packaging and CodeScene; two unavailable private fixtures are recorded as exclusions. TASK-22 remains To Do for deferred C#, AI explanations, PR comparisons and recursive inlining.
<!-- SECTION:FINAL_SUMMARY:END -->
