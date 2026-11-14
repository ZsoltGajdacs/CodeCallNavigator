# Function flowcharts

Select a Java, TypeScript or supported C# method and choose **Open flowchart** in
the inspector.
The workspace shows the method's source-derived control flow beside its analyzed
source snapshot. Select a step to highlight its source lines and inspect call
targets. Known project targets open their own charts; overload signatures,
certainty and analyzer provenance distinguish candidates. External or unresolved
calls explain why a project target cannot be followed.
Compiler-derived steps without a source location keep the source panel in place
and explain why no line is highlighted. Targets known to have no available body
show a disabled flowchart action and a reason; other unavailable or older saved
flows explain their limitation when opened.

Use **Previous function** (or Alt-Left) to return through flow history. **Back to
call graph** (or Escape) restores the call graph with its selection, filters,
history and camera unchanged. Flow camera and selection are restored when going
back through functions. Drag to pan, use the wheel/pinch to zoom, and choose
**Fit** to frame the chart. Drag the source divider to resize the source pane.
Source text can be selected and copied.

The source panel reads the immutable generation's cached source, not current disk
contents. A stale graph notice means a refresh is needed to show source changes.
Opening or following charts does not compile the project. Older graph caches remain
usable for graph exploration; refresh them to collect function-flow metadata.
AI explanations, PR comparisons and recursive call inlining are tracked
separately in TASK-22.

## C# support

Refreshing with the current analyzer collects C# flows for a bounded synchronous
subset: ordinary evaluation, branches, loops, simple switches, transfers and the
structured cleanup cases below.
Explicit accessors, local functions and top-level bodies retain their compiler
identities. Queries use exact analyzed UTF-8 source, including a BOM; accepted
non-UTF-8 contexts retain graph/source access with an explicit flow-unavailability
diagnostic. Compiler-reported encoding facts govern each linked compile context.

| Construct | Supported structure | Conservative or opaque boundary |
| --- | --- | --- |
| `try` / `catch` / `finally` | Pending return, throw and loop exits pass through applicable cleanup; a throwing finally replaces pending completion | Typed catches are candidate handlers with possible uncaught paths; bare catches are catch-all |
| `using` statements and declarations | Capture once, acquire left to right, dispose acquired resources in reverse order; declarations protect their remaining lexical scope, including top-level statements | Standard implicit `IDisposable` conversion and a proven reference/value/nullable category are required; dynamic, ref-like and pattern-only disposal remain opaque |
| Null and nullable resources | Implicit non-null or `HasValue` guards; a compiler-proven constant null has no disposal operation | Charts show guards, not general value/dataflow simulation; synthetic checks do not invoke overloaded source operators |
| Synchronous `foreach` | Evaluate collection and acquire enumerator once; protect `MoveNext`, `Current`, body and disposal on abrupt exits | Both compiler conversions must be identity conversions; proven disposable, sealed non-disposable and runtime-`IDisposable` cases remain distinct. Arrays, strings, ref/deconstruction, dynamic, ref-like and type-parameter enumerator routes remain opaque |
| Ordinary object `lock` | Capture once; modern `Monitor.Enter(object, ref bool)` is inside try, with guarded `Exit` in finally | The modern member contract must resolve; legacy/ambiguous members and `System.Threading.Lock` scope lowering remain opaque |
| Catch filters | Opaque dispatch region with explicitly synthetic normal/exceptional outcomes, propagation and applicable finally | No eager filter calls or invented unconditional handler; filter-before-unwind evaluation is not expanded |
| Guarded patterns and `goto` | Visible partial/opaque control regions | No guessed exact transfer destinations |
| Async and iterators | Whole-callable opacity for suspension syntax, including top-level await keywords | No scheduling or Java switch-yield interpretation; deferred local/lambda declarations do not make an ordinary outer body async |
| Deferred callbacks | Local functions retain separate callable identity; declaration is not eager execution | Anonymous callback invocation timing remains unexpanded |

Implicit disposal, enumeration and Monitor operations are visibly synthetic,
source-owned facts. They carry compiler-lowering diagnostics without inventing
inter-method graph nodes, relationships or resolved targets. Their possible
exceptional paths remain conservative. Unsupported regions are partial and do not
claim a complete simulation of arbitrary effects or transfers.

C# desktop/native/package workflow acceptance is recorded in the
[desktop evidence](../evidence/flowcharts/csharp/desktop/README.md). See the
[CLI contract](cli-queries.md),
[ingestion evidence](../evidence/flowcharts/csharp/ingestion/README.md) and
[cleanup evidence](../evidence/flowcharts/csharp/cleanup/README.md).

## What the paths mean

Charts show source control structure, not observed execution. Branches, loops,
labeled exits, switches, short-circuit expressions, returns and explicit throws
retain their source meaning. Effectful TypeScript switch tests execute in order
until a match; fallthrough skips further case-test evaluation.

Early returns, throws, breaks, continues and switch yields pass through applicable
`finally` and resource cleanup. Cleanup is duplicated by pending completion where
necessary to prevent a return path from resuming at an ordinary continuation.
A return or throw inside `finally` overrides the pending completion. Resource
initialization failure closes only resources already acquired.

Implicit call exceptions inside protected regions are conservative possibilities.
Java catch-type dispatch is not a complete runtime type simulation: candidate
handlers and uncaught routes are labeled and the flow is marked partial. The
charts do not enumerate every implicit exception, suppressed-exception detail,
thread schedule or side effect of an arbitrary expression.

Nested functions and callbacks appear in separate components. Their lexical
presence does not mean they run eagerly or asynchronously. For generic callbacks,
invocation timing is not inferred. TypeScript async/generator scheduling and
unsupported conditional constructs remain explicit opaque/partial regions rather
than having their nested calls presented as certain eager execution.

## Reactor

Compiler-confirmed `reactor.core.publisher.Mono` and `Flux` operations recognize
`defer`, `map`, `flatMap`, `filter`, `switchIfEmpty`, `onErrorResume`, `then`, `zip`,
`when` and `collectList`. Stage labels identify deferred/subscription callbacks,
empty/error alternatives and potential concurrency. Publisher assembly and callback
bodies remain separate; eager argument evaluation remains on the assembly path.
Application methods with the same names do not acquire Reactor behavior.

These source diagrams do not simulate subscription count, demand, scheduler choice,
interleaving or cancellation. Unknown operators remain opaque and visibly diagnosed.
Reactive callback components describe possible execution contexts, not one guaranteed
sequence through a pipeline.

## Availability and limits

Unavailable bodies, excluded symbols and ambiguous Java lambda attribution do not
produce guessed charts or guessed links. Source and call targets retain build and
project ownership, including duplicate type names in different compile contexts.
C# flow is unavailable when one native graph ID aliases distinct compiler
declarations, or when the body file differs from the retained declaration file,
as can happen with cross-file partial methods. The method graph remains available.

Export is bounded to 24 nested source levels and 1,500 IR nodes per callable.
Exceeding export limits produces partial metadata with explicit unsupported steps
so a deeply nested source file does not break the method graph. Constructed charts
are capped at 1,500 steps and 6,000 links, and source display at 16 MiB. A chart
that exceeds those limits reports the limit instead of silently truncating paths.

## Implementation boundaries

The Roslyn adapter, javac inventory and TypeScript compiler worker export detached,
versioned statement trees during refresh. They reuse compiler contexts and call
binding. The shared flow library owns completion semantics, binding cleanup and
Graphviz geometry. Source spans are half-open UTF-8 byte ranges over the exact
cached text, converted from compiler UTF-16 positions at export.

Flow metadata is separate from method nodes and inter-method relationships. Graph
composition and symbol pruning preserve those original relationships. The UI owns
background jobs and separate navigation, rejects obsolete generation/method results,
and shares source-row rendering and connector drawing with existing views. Layouts
share Graphviz process execution while keeping their layout policies separate.

See [verification evidence](../evidence/flowcharts/README.md) for real compiler
traces, graph-invariance comparisons, cache/refresh measurements and UI captures.
