# Standalone engine reference

The pre-refactor reference is local `master` merge
`fb54f5f789d94c6cbc5fe86bf20010edea815b33` (parents preserve the previous master
and `codex/function-flowcharts` at `8c69c6e`). Implementation starts on
`codex/standalone-graph-engine`. No remote branches were changed.

The complete release executable and matching adapters were frozen at
`/private/tmp/codegraph-reference-fb54f5f/`, using the app resource layout.
Rebuild from the reference commit with `tools/bootstrap.py` and
`cargo build --release --locked --bin codegraph`; copy the executable and the
CallAtlas, Java-bytecode, TypeScript, Gradle metadata and Maven metadata resources
together, as `tools/package.py` does. The temporary copy is a convenience; the
commit and checked-in dependency lock/patches are the durable reference.

Baseline verification:

- Dependency revisions and complete source patches match `analyzers.lock.json`.
- `cargo test --all-targets` passed.
- Python suite: 50 passed. TypeScript real-compiler suite: 46 passed.
- Including ignored library tests: 50 passed; the billing compiled-output test
  requires external `CODEGRAPH_COMPILED_METADATA` and could not run.
- Strict Clippy has four pre-existing warnings: a range loop in
  `src/edges/viewport.rs`, and constant assertions in the UI performance tests.
- CodeScene pre-commit safeguard passed with no modified files.
- No `.sonar/config.json` is configured.

The corpus manifest specifies existing authored fixtures rather than generated
build products. Full comparison includes flow and architecture payloads as well
as nodes, edges, entries, diagnostics and freshness facts. Build-generated content
volatility produces an explicit differing-field result; it is never silently
removed from graph semantics. The TypeScript fixture records the installed exact
direct dependency versions; the runner uses existing dependencies and never
installs packages. Each analysis additionally records evaluated engine versions.
