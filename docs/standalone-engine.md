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
