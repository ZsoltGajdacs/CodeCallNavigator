# Standalone graph engine

The public terminal command is `ccnavigator`, available on `PATH`. It exposes
the same command and JSON contracts as `codecallnavigator-engine`; the latter
remains the internal executable used by desktop and MCP clients. The standalone
engine package includes both binaries beside the same analyzer resources.
The agent skills use the independent [MCP adapter](mcp.md), configured for an
explicit project and invoking `ccnavigator` from PATH without a binary path.

Run the standalone package's `install.py` to expose the command in your terminal.
See [installation, updates and removal](cli-installation.md) for default and
custom-prefix instructions.

See the [external consumer contract](graph-contract.md) and its
[JSON Schema](graph.schema.json) for compatibility and input verification.
The [public CLI query interface](cli-queries.md) provides bounded graph queries,
source/flow data and generation-bound snapshots without running analysis.
Its optional `flow_comparison` operation reads selected before/after charts and
cached source directly from saved Git artifacts, retaining exact endpoint
digests and explicit per-side availability. It uses the same comparison binding
as `compare`, with exact stable-ID correspondence and an explicit manual
counterpart option; see the [saved-revision contract](cli-queries.md#saved-revision-function-flows).

## Project artifact and Git history

```sh
ccnavigator discover /checkout
ccnavigator analyze --project /checkout backend/pom.xml frontend/tsconfig.json
ccnavigator watch --project /checkout backend/pom.xml frontend/tsconfig.json
ccnavigator clear-cache --project /checkout
ccnavigator compare --project /checkout --base HEAD~1 --head WORKTREE
```

With no explicit targets, analysis selects the same default builds as discovery.
The saved graph uses a version-2 bundle. `/checkout/.codecallnavigator/graph.json`
is a small manifest referencing two immutable Zstandard-compressed objects under
`objects/`: structure and function flows. Commit the manifest and its referenced
objects alongside the source changes. Publication replaces the manifest atomically;
failed or cancelled analysis preserves the previous generation. Neither command
stages files or creates commits. The publisher retains the current and previous
generations and prunes older owned objects; active readers keep captured handles.

Structure contains nodes, relationships, entry points, diagnostics, architecture,
analyzed source snapshots and analysis provenance. Flows contain function bodies
and control-flow inventories. Both preserve stable symbol references, certainty,
provenance and ordered control flow. Compression is deterministic Zstandard level
9. The manifest's exact-byte BLAKE3 digest identifies the complete generation.
Treat both objects as project source information when sharing them.

Ordinary navigation loads only structure. Flow queries load the flow inventory on
demand within a CLI session; it can be explicitly released and loaded again. The
desktop releases hidden flow inventory and presentation caches after 60 seconds
without a visible flow view. The CLI owns no idle timer.

This is a clean format break: existing single-file graphs require reanalysis.
Old-format Git revisions report an unsupported-format error. Comparisons between
new-format revisions read their manifests and objects directly from the same
resolved commits, without checkout or fetch. Full comparison includes flows.
Exit status is 0 for equality, 1 for changes and 2 for operational/schema errors.

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
Java lambdas use source fingerprints where source-to-bytecode attribution is
unique. Ambiguous same-line or overlapping lambdas, local/anonymous type owners
and missing debug locations retain native IDs with a stability diagnostic; exact
correlation is tracked by TASK-32. Editing anonymous bodies may change their
identity. Native analyzer bindings
remain intact internally. All flow and entry references use the same exported
mapping. Collisions and unmapped owned paths reject publication.

Generated graph, lock and temporary publication files are not analysis inputs.
`.codecallnavigator/layers.yml` remains configuration; changing it is still detected.
The desktop SQLite store is a derived cache for navigation and recovery, not the
canonical graph history.

## Executables and process protocol

Build just the engine with `cargo build --locked -p codecallnavigator-engine --bin
codecallnavigator-engine`; it does not compile Rune or WGPU and does not need Graphviz.
`python3.13 tools/package.py --engine-only` creates a relocatable
`dist/CodeCallNavigatorEngine` with matching analyzer resources. Normal app packaging
includes that engine beside the desktop executable. The analysis toolchains and
project dependencies documented in the main README are still required.

Development can select `CODECALLNAVIGATOR_ENGINE` explicitly. `CODECALLNAVIGATOR_RESOURCES`
selects an analyzer resource root when validating a frozen distribution.
Executable and adapters must come from the same revision.

The desktop uses the version-2 `serve-once` newline-delimited JSON protocol.
Runtime observations and metrics travel separately from the artifact. The client
verifies the protocol version and artifact digest before caching the result.
Cancellation or a disconnected client stops analysis and its owned subprocesses.

Analysis requests may negotiate `"failure_version":1` to receive a typed `code`
on `failed`: `busy`, `inputs_changed`, `generation_changed`, `cancelled` or
`failed`. Requests omitting this field retain the existing message-only failure
envelope. Unknown failure versions are rejected before analysis. Optional
`expected_generation` binds an analysis request to the current canonical digest;
the engine checks it while holding the analysis lock, before build preparation.
A mismatch reports `generation_changed` without replacing the artifact. Manual
analysis omits this precondition.

`ccnavigator watch` starts continuous analysis for the chosen builds, including
in a checkout without a saved graph. The engine waits for edits to settle,
keeps validated compiler preparation, and follows new saved inputs. It publishes
only a complete, consistent graph. Stop the session with Ctrl-C.

Machine clients can open the same session with `ccnavigator analysis-session`.
Keep stdin open and send one newline-delimited request. Capabilities advertises
the command and its version under `analysis_session`.

```json
{"version":1,"project":"/checkout","roots":[],"targets":[{"path":"/checkout/backend","kind":"maven","language":"java","label":"Backend"}],"infer_linked_roots":false,"expected_generation":null}
```

A null `expected_generation` requests initial analysis. A digest follows the
matching saved selection. Stdout carries versioned activity frames with
monotonic sequence numbers. Publication reports a digest and leaves the
session open. Send `{"command":"cancel_refresh","version":1}` to cancel the
current attempt. Close stdin to stop the session and its owned processes.
`observe --auto-refresh` forwards activity from this same engine session.

Protocol version 2 also accepts
`{"command":"clear_cache","version":2,"project":"/checkout"}`.
The engine responds with its usual `hello`, followed by
`{"event":"cache_cleared","removed_files":1}` on success, or `failed` on an
operational error. `removed_files` counts the cache database and any sidecars
actually deleted; an already empty cache succeeds with zero. Older engines that
lack this additive command reject it as an invalid request.

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
`possible` certainty, `codecallnavigator.observer.v1` provenance and source evidence for the
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
edges with `codecallnavigator.callable.v1` provenance only at actual calls; argument and
return references do not become invocations. Graphs and flowcharts share targets
and diagnostics. Return environments are specific to each call.

Mutable/property paths, spread/rest forwarding, captured factories, optional
factories and explicit possibly-nullish default arguments remain unresolved.
See `tools/typescript/README.md` for the supported subset and
[internal adaptation rationale](<../.backlog/docs/architecture/doc-17 - Resolution-adaptation-rationale-and-attribution.md>) for upstream solution references.

## Spring event dispatch

Java analysis connects compiler-resolved `ApplicationEventPublisher.publishEvent`
sites to supported `EventListener`, `TransactionalEventListener` and
`ApplicationListener` methods in the same compiled source context. Relationships
use kind `spring-event`, possible certainty and `codecallnavigator.spring.event.v1`
provenance. Flow targets share their exact source call spans. Evidence preserves
conditions, transaction phase/fallback settings and async annotations; these do
not prove bean registration or runtime execution.

Recognized listeners remain framework entries without a publisher. Catalog
membership alone creates no invocation edge. Raw or erased generic payloads,
directly parameterized payload classes whose type arguments disappear at runtime,
lambda publishers, custom composed annotations and cross-context wiring remain
unresolved. A concrete nongeneric subclass can retain generic superclass metadata.
Runtime analysis requires the matching v6 Java source adapter, including its HTTP inventory capability.

## C# evaluated build inputs

Freshness records observed source/import membership and content. Changes during
analysis reject publication and preserve the previous graph. Build outputs are
excluded from directory scans, while evaluated imports remain explicit inputs.

The pinned native Roslyn/CallAtlas integration has a separate limitation in the retained
delimiter/percent solution-path probe: a SolutionDir-expanded Import produces only the
unconditional method, while a relative design-time Import produces the expected
conditional method as well. Both imports are observed by the preflight inventory. This
change fixes the CLI property boundary and does not repair that native import-expansion
behavior; see the
[probe](<https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/csharp-import-fix/literal-property-probe.json>). Ordinary
SolutionDir-expanded imports remain covered by real compiler acceptance.

The JSON version report in `analysis.engine_versions` records `csharp_input_policy` as
`msbuild-preprocess.v2`. The contexts follow the pinned [Roslyn BuildHost
source](https://github.com/dotnet/roslyn/blob/35d9211b841e7613c1d2f8f5af6d628ace696c4c/src/Workspaces/MSBuild/BuildHost/Build/ProjectBuildManager.cs#L25-L52).
The .NET design-time host uses the NonExistentFile sentinel and does not set
BuildingInsideVisualStudio. A version-2 bundle whose C# provenance lacks the current
policy remains readable, but verification reports `legacy_csharp_input_policy`
and `unverifiable` until explicitly refreshed. Old saved formats require reanalysis. Known
changed inputs still take precedence as `stale`. This policy check applies to C# nodes
or C# build targets, including valid empty graphs; Java/TypeScript artifacts are
unaffected.

Freshness verifies observed inputs only. SDK/external inputs outside declared logical
roots produce `unmapped_input` and `unverifiable`; use explicit `--root` mappings when
those inputs must be verifiable. Imports skipped by a false condition and absent
conditional-resolution candidates are not recorded probes. The repeated evaluation
detects membership changes during analysis, including normal/design imports swapping
while their union stays unchanged. Creating an unobserved conditional input after
publication is outside this guarantee. An empty external Compile glob returns no source
item from which to derive a directory scope; its first later file is likewise unobserved
unless a known project/import or recorded source scope covers it. This differs from an
empty referenced project, whose directory is explicitly observed. No arbitrary MSBuild
expression/glob parser is used to claim otherwise.

## MediatR dispatch

C# analysis recognizes compiler-bound `ISender`/`IPublisher` contracts, including
inherited `IMediator` calls. Actual argument types determine request/notification
types; response type arguments and source argument order do not select handlers.
Closed handler interfaces and their real `Handle` implementations must be visible
in the caller's compilation scope. Requests require one matching handler;
notifications may have several. Existing native relationships remain intact.

Added calls have possible certainty and `codecallnavigator.mediatr.v1` provenance.
Evidence records the operation, concrete types, source site and Task completion
contract. `mediatr.direct_await` means directly awaited with only parentheses;
`mediatr.conditional_context` reports surrounding conditional syntax, not a proven
path guard. Neither a Task return nor these flags prove asynchronous scheduling.
Recognized closed handlers remain framework entries without dispatch calls.

Object/interface-erased payloads, open types, anonymous callables, ambiguous
requests and handlers outside compiler-visible scope remain unresolved. Concrete
`Mediator` method declarations and custom wrappers are outside the initial
contract-only subset. The matching
CallAtlas adapter must provide framework-facts capability v1, including snapshot
round trips; stale/incomplete facts fail explicitly.

### C# source flows

Fresh C# analysis additionally requires `flowFactsVersion: "2"`, a version-1
`flowInventory`, and `sourceFactsVersion: "1"`. Compiler source facts include a
nullable `sourceCodePage`. Existing node identities and evaluated compile owners
bind functions and targets; exact UTF-8 originals bind source spans. Explicit
non-UTF-8 contexts remain graph/source-only. Missing, incompatible or corrupt
required facts reject publication and preserve the prior artifact generation.
Historical graphs remain readable and explain when a refresh is needed for flows.
See [function flowcharts](function-flowcharts.md) for the supported subset and
current semantic limits.

## Navigation and HTTP

Compiler-bound Next App Router navigation and builtin TypeScript `fetch` create
possible `navigation` or `http` boundary relationships. They do not imply a
synchronous call or a proven deployment. Routes come from Next App Router exports,
qualified Spring Web annotations and ASP.NET MVC controller metadata. Known
unsupported conditions, catchalls and route orders remain ambiguity blockers.
Compiler-qualified custom ASP.NET metadata providers and MVC conventions that
can rewrite routes also block affected application owners, including awaitable
handler results and parameter/property hooks. Analysis never executes those hooks.

Cross-language HTTP requires the owning project's `.codecallnavigator/boundaries.yml`.
Build selection alone does not establish a shared origin. Configuration examples,
application ownership and the deliberately bounded matching rules are documented
in [the boundary contract](../crates/codecallnavigator-core/src/boundary/README.md).
Facts and explicit flow target kinds survive canonical export and the derived
desktop cache. Next Link JSX adds a graph boundary without inventing a flow call.

Only root-relative literal URLs and supported literal request methods are joined.
Shadowed/replaced APIs, mutable or escaped routers, dynamic URL/options and
unmodeled deployment/route behavior stay unresolved. Next configuration or
middleware/proxy presence conservatively blocks its route candidates; the
analyzer does not execute project configuration.

## Inference cache

Analysis still extracts and validates fresh compiler facts. The disposable
`.codecallnavigator/inference.sqlite3` cache reuses selected inference decisions;
it is not a graph history or a substitute for compiler analysis. Commit the single
graph file and authored configuration, not this derived cache. Missing or malformed
cache data falls back to full inference with a warning. Failed or cancelled
analysis preserves the previous graph and cache.

Use `codecallnavigator-engine clear-cache --project /checkout` or the desktop's
**Clear cache** button beside analysis to discard the inference database and its
exact `-wal`, `-shm` and `-journal` sidecars. Clearing holds the same project
analysis lock and fails while another analysis owns it. Nonregular or hard-linked
cache paths and a symlinked `.codecallnavigator` directory are rejected before deletion.
Corrupt regular cache files can be removed so the next analysis recomputes all
inference decisions. Clearing preserves `graph.json`, the displayed graph,
navigation, authored configuration and build outputs; start analysis separately
after it completes.

Set `CODECALLNAVIGATOR_DISABLE_INFERENCE_CACHE=1` to run the full reference path without
reading or writing the cache. Canonical graph bytes are identical in both modes.
Cache hit/miss/rejection counters and warnings use progress/performance reporting,
not canonical graph content.
