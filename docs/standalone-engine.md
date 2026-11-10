# Standalone graph engine

`codegraph-core` owns shared graph/flow/architecture contracts and pure graph
operations. `codegraph-engine` owns the coordinator and compiler-backed language
adapters. The desktop owns presentation and workspace caching and invokes the
engine executable rather than calling analyzer functions.

## Project artifact and Git history

```sh
codegraph-engine discover /checkout
codegraph-engine analyze --project /checkout backend/pom.xml frontend/tsconfig.json
codegraph-engine compare --project /checkout --base HEAD~1 --head WORKTREE
```

With no explicit targets, analysis selects the same default builds as discovery.
The only persistent graph JSON is `/checkout/.codegraph/graph.json`. Each
successful refresh atomically replaces it while holding the project analysis
lock. Failed or cancelled work preserves the previous graph. Neither command
stages files or creates Git commits. Commit the graph alongside the changes it
describes, then use Git to compare any two revisions; comparisons read blobs
without saving historical copies. Exit status is 0 for equality, 1 for changes,
and 2 for operational/schema errors.

The versioned artifact includes nodes, relationships, entry points, diagnostics,
source-derived flow inventories and architecture facts. Relationships refer to
stable symbol IDs rather than array positions. Unordered sets are sorted;
control-flow statement order is preserved. Complete source facts can make the
artifact large. Treat it as project source information when sharing it.

Paths use `@repo/` relative to the output-owning project. Add repeated
`--root NAME=PATH` mappings for project-owned sources outside that root; use the
same logical names in every checkout. Absolute checkout paths, temporary paths,
fingerprint observation times and performance metrics are excluded from the
artifact. Toolchain versions and input observations are runtime report data.
Reproducibility assumes equal sources, dependencies, selections and toolchains;
generator output changes remain observable rather than being silently masked.

Named exported identities use logical ownership and compiler signatures.
TypeScript adds enclosing declaration and callable role; anonymous functions use
token fingerprints that ignore trivia and deterministic sibling disambiguation.
Editing anonymous bodies may change their identity. Native analyzer bindings
remain intact internally. All flow and entry references use the same exported
mapping. Collisions and unmapped owned paths reject publication.

Generated graph, lock and temporary publication files are not analysis inputs.
`.codegraph/layers.yml` remains configuration; changing it is still detected.
The desktop SQLite store is a derived cache for navigation and recovery, not the
canonical graph history.

## Executables and process protocol

Build just the engine with `cargo build --locked -p codegraph-engine --bin
codegraph-engine`; it does not compile Rune or WGPU and does not need Graphviz.
`python3.13 tools/package.py --engine-only` creates a relocatable
`dist/CodeGraphEngine` with matching analyzer resources. Normal app packaging
includes that engine beside the desktop executable. The analysis toolchains and
project dependencies documented in the main README are still required.

Development can select `CODEGRAPH_ENGINE` explicitly. `CODEGRAPH_RESOURCES`
selects an analyzer resource root when validating a frozen distribution.
Executable and adapters must come from the same revision.

The desktop uses the versioned `serve-once` newline-delimited JSON protocol.
Runtime observations and metrics travel separately from the artifact. The client
verifies the protocol version and artifact digest before caching the result.
Cancellation or a disconnected client stops analysis and its owned subprocesses.
`analyze-native` is an internal acceptance oracle retained for comparison with
the pre-extraction SQLite implementation; it writes no persistent graph file and
is not the supported project export interface.

## Adding another language

Keep new support within the existing explicit adapter dispatch:

1. Add language/build discovery and default selection rules.
2. Implement preparation and analyzer execution under the language adapter.
3. Normalize actual analyzer symbols, ownership and relationships into shared
   contracts. Supply semantic export keys where native IDs depend on positions.
4. Extend runtime resources, bootstrap locks and packaging together.
5. Add real-toolchain acceptance fixtures, invalid-input/cancellation checks and
   corpus comparisons covering flow and architecture facts where supported.

The shared coordinator owns lifecycle, composition, snapshots and publication.
Build tools stay separate from graph analysis. Framework inference belongs to
the relevant language adapter. This extraction introduces no dynamic plugin
framework and does not substitute name matching for compiler bindings.

Run `cargo test --workspace --all-targets` so moved package tests remain covered.
The frozen baseline and reproducible corpus are described in
`evidence/engine/README.md` and `evidence/engine/corpus.json`.

## Relationship evidence

Inferred relationships include versioned rule evidence: the source construct,
registration location when known, supporting callable identities, channel and
execution semantics. These fields survive Git comparison, workspace caching and
the selected-relationship inspector. Evidence sites are source constructs, not
observations of runtime execution. References, deferred callbacks and possible
invocations remain distinct. Old graphs without evidence remain readable.
Optional `qualifiers` retain rule-specific restrictions as opaque string values;
symbol and path references use supporting identities and source sites instead.

## Observer collections

TypeScript observer inference connects a real callback invocation in a `for...of`
loop with compiler-bound registrations into the same callback `Set`. Edges carry
`possible` certainty, `codegraph.observer.v1` provenance and source evidence for the
invocation and registrations. Multiple registrations of the same callback retain
all evidence without multiplying the invocation count. Registration alone does
not establish invocation, and flowchart targets use the same resolver as graph edges.

The initial subset covers module-local constant collections, module/namespace
private static readonly fields and private readonly fields on closed singleton
classes. A singleton needs a nonexported top-level module class with no inheritance
and one construction initializing a top-level constant. Registrar and dispatcher receivers must
trace to that constant, immutable aliases or same-class method `this`. Static
storage must use the lexical class or an immutable exact-class alias, rather than
dynamic static `this`. Multiple
instances, factories, exported or escaping constructors, untyped instance escapes
and mutable collection access remain unresolved with diagnostics. Contextual
type-changing aliases, parameters, containers and returns defer inference because
method writes can bind to different source declarations. Extracted or rebound
dispatcher methods are also deferred. Method callbacks
require a stable static or const object-literal receiver. Decorated storage and
method implementations, and known writes to built-in Set storage operations,
disable inference because compiler symbols alone cannot establish their behavior.

Ordinary noncallback Sets are unaffected. `Set.forEach` and nested deferred
functions are not reconstructed. Channel names are explanatory labels; internal
matching uses exact compiler declarations, so identically named fields in
unrelated classes or namespaces do not join.

## Callable values

TypeScript callable inference follows immutable `??`/`||` aliases, direct
callable parameters and simple returned functions. It adds `possible` invocation
edges with `codegraph.callable.v1` provenance only at actual calls; argument and
return references do not become invocations. Graphs and flowcharts share targets
and diagnostics. Return environments are specific to each call.

Mutable/property paths, spread/rest forwarding, captured factories, optional
factories and explicit possibly-nullish default arguments remain unresolved.
See `tools/typescript/README.md` for the supported subset and
`docs/engine-adaptations.md` for upstream solution references.

## Spring event dispatch

Java analysis connects compiler-resolved `ApplicationEventPublisher.publishEvent`
sites to supported `EventListener`, `TransactionalEventListener` and
`ApplicationListener` methods in the same compiled source context. Relationships
use kind `spring-event`, possible certainty and `codegraph.spring.event.v1`
provenance. Flow targets share their exact source call spans. Evidence preserves
conditions, transaction phase/fallback settings and async annotations; these do
not prove bean registration or runtime execution.

Recognized listeners remain framework entries without a publisher. Catalog
membership alone creates no invocation edge. Raw or erased generic payloads,
directly parameterized payload classes whose type arguments disappear at runtime,
lambda publishers, custom composed annotations and cross-context wiring remain
unresolved. A concrete nongeneric subclass can retain generic superclass metadata.
Runtime analysis requires the matching v6 Java source adapter, including its HTTP inventory capability.

## MediatR dispatch

C# analysis recognizes compiler-bound `ISender`/`IPublisher` contracts, including
inherited `IMediator` calls. Actual argument types determine request/notification
types; response type arguments and source argument order do not select handlers.
Closed handler interfaces and their real `Handle` implementations must be visible
in the caller's compilation scope. Requests require one matching handler;
notifications may have several. Existing native relationships remain intact.

Added calls have possible certainty and `codegraph.mediatr.v1` provenance.
Evidence records the operation, concrete types, source site and Task completion
contract. `mediatr.direct_await` means directly awaited with only parentheses;
`mediatr.conditional_context` reports surrounding conditional syntax, not a proven
path guard. Neither a Task return nor these flags prove asynchronous scheduling.
Recognized closed handlers remain framework entries without dispatch calls.

Object/interface-erased payloads, open types, anonymous callables, ambiguous
requests and handlers outside compiler-visible scope remain unresolved. Concrete
`Mediator` method declarations and custom wrappers are outside the initial
contract-only subset. C# flow inventories remain unavailable. The matching
CallAtlas adapter must provide framework-facts capability v1, including snapshot
round trips; stale/incomplete facts fail explicitly.


## Navigation and HTTP

Compiler-bound Next App Router navigation and builtin TypeScript `fetch` create
possible `navigation` or `http` boundary relationships. They do not imply a
synchronous call or a proven deployment. Routes come from Next App Router exports,
qualified Spring Web annotations and ASP.NET MVC controller metadata. Known
unsupported conditions, catchalls and route orders remain ambiguity blockers.

Cross-language HTTP requires the owning project's `.codegraph/boundaries.yml`.
Build selection alone does not establish a shared origin. Configuration examples,
application ownership and the deliberately bounded matching rules are documented
in [the boundary contract](../crates/codegraph-core/src/boundary/README.md).
Facts and explicit flow target kinds survive canonical export and the derived
desktop cache. Next Link JSX adds a graph boundary without inventing a flow call.

Only root-relative literal URLs and supported literal request methods are joined.
Shadowed/replaced APIs, mutable or escaped routers, dynamic URL/options and
unmodeled deployment/route behavior stay unresolved. Next configuration or
middleware/proxy presence conservatively blocks its route candidates; the
analyzer does not execute project configuration.

## Incremental inference

Each run still extracts and validates fresh compiler facts. Observer, callable,
Spring and MediatR inference currently recompute from those facts. The boundary
pass reuses dependency-keyed routing decisions in the disposable project-local
`.codegraph/inference.sqlite3`; it never loads prior graph edges as evidence.
The SQLite file and its exact journal/WAL/SHM names are excluded from discovery,
source fingerprints and refresh triggers. Continue committing only the single
graph file and authored configuration, not this derived cache.

A decision depends on its current source use, configured application scope,
toolchain, complete matching route/blocker membership and supported symbols.
Negative lookups track empty membership, so adding a matching handler invalidates
a cached miss. An automatic implementation digest invalidates decisions when the
rule implementation changes. Hits rebind current symbols and construct fresh
edge evidence and flow targets; unrelated node index changes cannot replay old
indexes. Flow attachment always checks the current exact source call site.

Cache reads are optional and read-only. Missing, incompatible or malformed data
falls back to full inference and reports a volatile warning. Unowned, corrupt or
nonregular cache files are preserved rather than overwritten; remove the derived
cache to rebuild it. Successful input verification and atomic graph publication
happen before the cache replaces its current rows in one transaction under the
same project lock. Cache persistence failure cannot turn a published graph into a
failed analysis. Failed or cancelled analysis preserves the successful graph and
previous cache. The cache stores no graph history.

Set `CODEGRAPH_DISABLE_INFERENCE_CACHE=1` to run the full reference path without
reading or writing the cache. Canonical graph bytes are identical in both modes.
Cache hit/miss/rejection counters and warnings use progress/performance reporting,
not canonical graph content.
