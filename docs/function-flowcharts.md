# Function flowcharts

Select a Java or TypeScript method and choose **Open flowchart** in the inspector.
The workspace shows the method's source-derived control flow beside its analyzed
source snapshot. Select a step to highlight its source lines and inspect call
targets. Known project targets open their own charts; overload signatures,
certainty and analyzer provenance distinguish candidates. External or unresolved
calls explain why a project target cannot be followed.

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
C# support, AI explanations, PR comparisons and recursive call inlining are
tracked separately in TASK-22.

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

Export is bounded to 24 nested source levels and 1,500 IR nodes per callable.
Exceeding export limits produces partial metadata with explicit unsupported steps
so a deeply nested source file does not break the method graph. Constructed charts
are capped at 1,500 steps and 6,000 links, and source display at 16 MiB. A chart
that exceeds those limits reports the limit instead of silently truncating paths.

## Implementation boundaries

The existing javac inventory and TypeScript compiler worker export detached,
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
