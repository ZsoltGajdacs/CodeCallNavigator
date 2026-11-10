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

## Foundation gate — 2026-11-10

The complete extraction matched the frozen native payload on Maven, Gradle, C#,
TypeScript and mixed workloads. `extraction-comparison.jsonl` records the first
check; `foundation-performance.jsonl` records three further comparisons per
workload against the frozen release and the packaged standalone release. Every
comparison matched. Only unordered graph collections and SQLite representation
details are normalized; statement/control-flow order stays strict.

The timing medians did not regress on this corpus. These measurements compare
headless native output against baseline SQLite publication; they do not measure
desktop rendering or claim a production speedup. Memory was not sampled in this
reference check.

`canonical-comparison.jsonl` records repeated and relocated exports (including
reversed selected-target order). The initial Gradle relocation test exposed
checkout-derived project display names; canonical export now maps that prefix to
the logical root and hydration restores the local display name. All five cases
then produced identical bytes.

Full workspace tests and strict workspace Clippy passed. Real-toolchain
acceptance passed with private billing-data tests and explicit GPU/CPU profiling
benchmarks excluded. Those prerequisites are unchanged from baseline. The four
pre-existing Clippy diagnostics were resolved without changing graph behavior.
Python tests passed (53 at foundation integration); TypeScript adapter and flow
tests passed, including semantic identities under comments and movement.

Both distributions were built, and the app was code-signed and verified.
`packaged-smoke.json` records identical output from packaged standalone analysis
and the packaged desktop compatibility CLI. Actual-executable tests cover invalid
protocol/version, EOF, idle SIGTERM, active cancellation, descendant reaping,
failed-publication preservation, linked sources and stalled terminal responses.
Detached-child forced cleanup was verified outside the macOS sandbox; that
specific regression is ignored by default because sandbox process-table access
is denied. Runtime reports when forced cleanup cannot be verified.

Each acceptance fixture now has one checked-in `.codegraph/graph.json`; the mixed
artifact belongs to `fixtures/`. Update them with
`python3.13 tools/update_engine_corpus.py --executable /path/to/codegraph-engine`,
then review changes with the engine's Git comparator before committing. The
updater never installs dependencies, stages files or creates commits.

Foundation review found and fixed two client boundary regressions: discovery
before desktop boot finished, and linked project-owned sources outside the
selected folder. The machine protocol now derives stable logical aliases from
compiler-owned paths for the desktop; explicit human CLI roots remain strict.
The legacy timing destination rejects graph publication paths and symlink aliases.
