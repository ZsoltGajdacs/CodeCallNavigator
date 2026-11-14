# TASK-22 investigation and delivery plan

Investigated 2026-11-14. This is a planning handoff; no runtime changes or new
runtime test results are claimed. The existing parent acceptance criteria remain
unchanged. Child tasks contain durable scope, outcomes, dependencies and completion
checks; their worker must recheck current code and record an execution plan when
starting them.

## Current implementation

- [Function flowcharts](../function-flowcharts.md) and
  [existing evidence](../../evidence/flowcharts/README.md) describe the delivered
  Java/TypeScript source-flow workspace and its limitations.
- `crates/codecallnavigator-core/src/flow/model.rs` defines the detached statement
  IR, source spans, targets and built charts. The inventory version is currently
  1. `builder/exceptions.rs` groups pending completions through cleanup; catches
  carry a parameter, catch-all flag and body, without a filter expression.
  `Yield` currently serves switch completion; it is not an iterator-suspension
  model. These distinctions matter for C#.
- `vendor/callatlas/src/CallAtlas.Core/CallGraphBuilder.cs` already collects body
  sites with compiler identities and reuses Roslyn semantic models during binding.
  This is the extraction integration point. `CodeMap.cs` and `CodeMapJson.cs`
  contain no flow inventory contract. Avoid a second parser or a separate
  name-based target resolver.
- `crates/codecallnavigator-engine/src/analyzers/csharp.rs` imports declaration,
  framework, HTTP and architecture facts, but no flow facts. `normalize_atlas`
  constructs identities and ownership before pruning. Any new function/call
  references must use the same mapping and survive artifact round trips.
- `crates/codecallnavigator-core/src/flow/lookup.rs` explicitly rejects C#.
  `query/operations.rs` builds charts from the selected immutable generation and
  validates cached source ranges. `query/contract.rs` offers only a symbol on its
  current `Flow` request, without revisions or expansion options.
- `src/flow_workspace.rs` obtains charts/source through graph queries, performs
  layout in background jobs and caches by epoch/symbol. It rejects obsolete jobs
  and owns separate flow history. Comparison, expansion and explanation state
  must add the relevant identities instead of reusing that cache indiscriminately.
- `artifact/revision.rs` already loads saved artifacts from Git commits or
  WORKTREE without checkout, refresh or fetch. `artifact/difference.rs` already
  matches flow inventories by exact function symbol. Engine comparison results
  bind both endpoint digests and commit identities. These are useful foundations,
  but they do not yet serve historical charts/source or a comparison workspace.
- Searching production source/configuration found no existing model-provider,
  credential or explanation-generation integration. Provider and source-sharing
  decisions therefore remain open, rather than being assumed implementation work.

## Delivery order and completion map

All children are tracked under TASK-22 as To Do and unassigned. This indicates
queued scope, not approval of the unresolved AI design. Dependencies express
technical prerequisites, not a requirement to build all four branches together.

| Task | Bounded result | Prerequisite | Parent criterion |
| --- | --- | --- | --- |
| TASK-22.1 | Versioned, bounded Roslyn flow export and reproducible upstream patch | TASK-21 (Done) | 1, 5 |
| TASK-22.2 | Normalized/persisted C# flow and production CLI query | TASK-22.1 | 1, 5 |
| TASK-22.3 | Validated synchronous cleanup and explicit unsupported boundaries | TASK-22.2 | 1, 5 |
| TASK-22.4 | Source-linked C# desktop workflow and packaged acceptance | TASK-22.3 | 1, 5 |
| TASK-22.5 | Revision-bound before/after flow and source CLI contract | TASK-58 (Done) | 3, 5 |
| TASK-22.6 | Before/after desktop workspace with endpoint provenance | TASK-22.5 | 3, 5 |
| TASK-22.7 | Shared bounded call-site expansion with identity and cycle handling | TASK-21 (Done) | 4, 5 |
| TASK-22.8 | Optional CLI/desktop expansion and collapse | TASK-22.7 | 4, 5 |
| TASK-22.9 | Reviewed explanation-provider and data/anchor decision | TASK-21 (Done) | 2, 5 |
| TASK-22.10 | Approved provider route with validated anchored generation | TASK-22.9 and explicit design decision | 2, 5 |
| TASK-22.11 | Optional explanation UI and failure/staleness behavior | TASK-22.10 | 2, 5 |

Recommended first delivery is TASK-22.1 through TASK-22.4. It completes the C#
workflow without waiting for AI, revision comparison or interprocedural expansion.
Comparison and expansion can first use the existing Java/TypeScript inventories.
Keep later UI integrations sequential when they touch the same workspace state.
The parent remains open until every criterion has objective delivery evidence;
neither a support matrix nor a provider spike alone completes a capability.

## C# semantic boundary

The initial exporter can land with a deliberately limited, explicitly diagnosed
subset. Desktop release follows semantic validation, rather than treating the
presence of serialized statements as proof of C# correctness.

| Area | Planned delivery rule | Verification |
| --- | --- | --- |
| Structured synchronous bodies | Straight-line evaluation, branches, loops, simple switches, returns/throws and short-circuit expressions; preserve actual evaluation order | Real Roslyn output plus executable marker traces |
| Callable/source attribution | Existing methods, explicit accessors, local functions and top-level identities where unambiguous; no invented callback identities | Overloads, assembly collisions, linked contexts, same-line sites, Unicode/CRLF/BOM |
| Exception and resource scopes | Pending completion passes through applicable finally, acquired-resource disposal and supported lock/foreach cleanup | Nested acquisition failure, return, break/continue, rethrow and throwing cleanup traces |
| Filtered catches and guarded transfers | Exact handling only where proven; otherwise preserve partial/opaque structure and possible escape paths | Filter/guard cases cannot become unconditional handlers or guessed destinations |
| Async, iterators and callbacks | No scheduling or eager-execution inference; iterator yield cannot reuse Java switch-yield meaning | Explicit partial/unavailable outcomes; detached callback bodies |
| Complex expressions and unsupported syntax | No flattened eager calls when evaluation order or conditionality is unknown | Conditional access/coalescing, patterns, goto, dynamic/other unsupported cases appear in the support matrix |

TASK-22.3 must publish the precise supported matrix and test it. Do not describe
the result as complete C# control-flow simulation. In particular, legal C# tests
should use a throwing finally to check replacement of a pending completion;
Java's return-from-finally fixture is not a valid C# fixture.

## Shared contracts and independent branches

Core owns reusable flow semantics and bounded expansion. Engine adapters own
compiler facts and process orchestration. Desktop owns presentation and jobs.
MCP remains an isolated CLI client; no core/engine dependency or graph reader is
introduced there. New MCP tools are outside this delivery scope unless separately
requested; existing MCP wire validators still need compatibility checks when
their shared CLI response shapes change.

Revision comparison uses saved artifacts and their analyzed source, including
when current files differ or are absent. Automatic correspondence uses exact
stable symbol identities. A rename/move remains a removal and addition; the UI
may let a user explicitly choose another counterpart and labels that choice.
Local base/head refs can represent PR revisions. Hosted PR lookup, credentials,
fetching and automatic historical compilation are outside these tasks. Missing
historical source/flow is an explicit per-side result, not a reason to read live
source as if it belonged to the old revision.

Expansion needs occurrence identity in addition to method identity. Each selected
call-site occurrence owns its source and path; recursion detection uses the
active path so repeated nonrecursive callees still work. Preserve completion and
exception behavior when splicing is supported, and expose unavailable/opaque
invocations where it is not. References, HTTP/navigation targets and callbacks
do not become synchronous calls. Depth and total size limits are visible outcomes.

Explanation anchors need artifact/function/chart identity and a step/source
reference, including duplicated cleanup steps. Provider text cannot change the
compiler graph or become source facts. TASK-22.9 must settle provider execution,
configuration/credentials, permitted outgoing context, initiation/disclosure,
storage/logging, budgets, cancellation, output validation and offline behavior.
Only then can TASK-22.10 be implementation-ready. Ordinary single-function charts
are the first target; revised/expanded views require correct anchors or an
explicit unavailable state. No external provider choice is approved by this plan.

## Verification and handoff

- Exporter changes require a checked-in patch, pinned checksum and application
  documentation against the current CallAtlas base in `analyzers.lock.json`.
  Test bootstrap installation, repeat use and migration from the exact previous
  patch. Baseline existing upstream failures before attributing regressions.
- Follow `tests/java_flows.rs`, `tests/flow_typescript.rs`, `tests/flow_core.rs`
  and `tests/flow_inventory.rs` for observable semantics, attribution and
  persistence checks. New .NET fixtures must exercise actual analysis and
  executable traces, not only deserialize hand-written IR. Trace membership alone
  is insufficient: assert absence of impossible fallthrough/cleanup paths too.
- Exercise CLI subprocesses for generation/revision binding and backward
  compatibility. Git fixtures verify no fetch/checkout/source/artifact mutation;
  expansion fixtures verify per-occurrence source identity and hard budgets.
  Provider fixtures are deterministic, with real-provider acceptance reported
  separately from mocked success.
- Desktop deliveries require real native workflows, source highlighting, Back,
  camera/scroll ownership, stale-result rejection, minimum-window light/dark
  captures and package/relocation evidence where resources change.
- Runtime work across these subsystems uses `./verify-full.sh` outside the macOS
  sandbox, with required isolated profiling inputs from the validation reference.
  Review modified source with CodeScene, safeguard before commit, and analyze the
  intended change set before PR. Report missing prerequisites and existing failures
  accurately. Do not use debug-profile ignored benchmark runs.
- This investigation changes only planning documentation and Backlog records;
  validate it with `./verify-light.sh` in documentation-only mode, task readback,
  dependency checks and link/diff checks. No build or analyzer benchmark is needed
  to claim the planning handoff complete.
