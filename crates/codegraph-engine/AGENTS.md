# Analysis engine practices

Read the workspace practices and mandatory policies in
[AGENTS.md](../../AGENTS.md). This guide applies to `codegraph-engine`.

## Responsibilities

The engine owns standalone command handling and compiler-backed analysis.
`entrypoints/` owns CLI/protocol policy; `analyzers/` owns orchestration, process
execution and language adapters. Depend on core contracts without depending on
the desktop, Rune or WGPU. Keep the standalone build independent of UI resources.

- Keep entry points thin. Share the production analysis pipeline between CLI and
  desktop protocol requests rather than duplicating analyzer behavior.
- Keep build-tool discovery/preparation separate from symbol normalization,
  graph analysis and framework inference. Keep language-specific rules with the
  relevant adapter; the coordinator owns lifecycle and composition.
- Normalize into core types at the adapter boundary. Add languages through the
  explicit dispatch and [extension contract](../../docs/standalone-engine.md),
  rather than introducing a speculative plugin framework.

## Analysis and process invariants

- Use compiler bindings and verified source/bytecode attribution for identity
  and ownership. Do not substitute name matching for resolved symbols.
- Preserve native bindings, relationship kinds, certainty, source spans and
  provenance. Keep possible invocations, deferred callbacks and references
  distinct; unresolved ambiguity should remain visible through diagnostics.
- Use the same resolver for graph edges and flow targets. Framework registration
  alone must not create an invocation or establish runtime execution.
- Preserve snapshots and fingerprints across preparation and analysis. Reject
  incomplete output, concurrent input changes and incompatible adapter resources
  rather than publishing a partial graph as a successful refresh.
- Propagate cancellation and client disconnects to owned subprocesses. One-shot
  commands receive closed stdin; persistent worker protocol pipes remain explicit.
  Keep machine-readable stdout free of logging and report failures contextually.
- Preserve the versioned process protocol, artifact digest verification and
  atomic publication contract. Failed or cancelled work retains prior results.
- Update resource capability/version checks, bootstrap locks and packaging
  together when adapter contracts change. Executables and adapters must agree.

## Verification

Run from the workspace root:

```sh
cargo check --locked -p codegraph-engine --all-targets
cargo test --locked -p codegraph-engine --all-targets
cargo build --locked -p codegraph-engine --bin codegraph-engine
```

Use real compiler/build fixtures for adapter regressions, including ambiguity,
unsupported input, cancellation and preservation of the previous artifact where
affected. Keep unit tests focused on isolated normalization or argument logic.
Run relevant ignored toolchain tests and corpus comparisons as documented in
[README.md](../../README.md) and [engine evidence](../../evidence/engine/README.md).
Resource/relocation changes also need engine packaging verification through
`python3.13 tools/package.py --engine-only`. Follow root scoped-formatting and
quality policies; report missing toolchains instead of claiming acceptance.
