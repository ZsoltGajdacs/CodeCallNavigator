# Git graph comparison acceptance

TASK-58 adds the version-1 `compare` query to both public CLI names and the
`compare_graphs` MCP tool. [The CLI contract](../../docs/cli-queries.md#git-graph-comparison)
documents arguments, field-level records, scope, snapshot binding and page limits.

## Behavioral verification

- Core tests exercise complete graph sections, opaque extensions, unordered
  collections versus ordered flow, certainty/provenance variants, exact identity,
  explicit null versus absence and graph/artifact namespace collisions.
- Temporary real Git repositories exercise immutable commit resolution, nested
  project paths, exact-byte digests and a missing promisor blob. Missing partial
  clone objects fail without fetching or creating `FETCH_HEAD`.
- Engine process acceptance covers stale saved working graphs, two committed
  revisions without a working artifact, branch movement with identical graph
  bytes, session parity, complete pagination, byte limits and structured errors.
- Real MCP stdio acceptance compares full CLI envelopes and pages and verifies
  committed freshness, errors, immutable input files and no cache writes.
- Separate real MCP cancellation/disconnect tests hold the actual CLI in a Git
  read, then verify both CLI and Git PIDs terminate and saved bytes remain intact.

The legacy standalone comparison retains its original DTO and exit behavior.
Comparison semantics, artifact retrieval, CLI dispatch, transport framing and MCP
wire validation have separate owners. Independent review findings concerning
extension namespaces and partial-clone fetching were fixed and regression tested.

## Large graph measurement

[Raw measurements](latency-memory.json) use a retained compiler-produced graph
with 10,827 nodes and 17,331 relationships. The baseline is 113,628,316 bytes;
the copied target changes one node label and adds one diagnostic. The source
project was not modified or analyzed. Raw inputs remain ignored under
`target/verification/task58-large-comparison/`, with the baseline in that temporary
repository's Git history and the target in its working graph. Private source
content is not included in this evidence directory.

Three sequential release `ccnavigator query` processes returned two records in a
7,492-byte envelope. Median elapsed time was 4.3998 seconds; maximum observed
resident memory was 4,017,881,088 bytes (about 3.74 GiB). Timing includes Git blob
retrieval, artifact validation, freshness observation, comparison and JSON output.
Filesystem/OS caches were uncontrolled. These controlled edits measure the
comparison machinery, not the size or cost of an actual historical change.

These are the initial implementation's measurements. Its canonical comparison
copies were subsequently removed as described below. Pages remain stateless and
re-read and compare their endpoints; a small response does not imply a small
working set.

## Memory optimization (2026-11-13)

[Before/after samples](memory-optimization.json) remeasure the same retained graph
pair and an additional case that changes one function diagnostic. The baseline
executable is retained alongside the inputs and full response envelopes under
`target/verification/task58-memory-optimization/`. Each case was measured with
three sequential release processes per executable using macOS `time -l`, after
full verification completed and was idle. The after executable is the verified
packaged CLI.

| Case | Before peak RSS | After peak RSS | Reduction | Before median | After median |
| --- | ---: | ---: | ---: | ---: | ---: |
| Node label and graph diagnostic | 3.73 GiB | 2.07 GiB | 44.4% | 4.69 s | 2.91 s |
| Same pair plus one function diagnostic | 5.18 GiB | 2.07 GiB | 60.0% | 5.11 s | 2.95 s |

Peak figures are the maximum across the three samples, and times are medians.
Filesystem caches and other machine activity were uncontrolled. The earlier
instrumented investigation's flow-change RSS is not used as this table's baseline;
both columns here use uninstrumented release CLI processes on identical files.

The CLI consumes loaded artifact graphs for canonicalization instead of retaining
two additional full graph trees. Graph and provenance validation deserialize
typed values from borrowed JSON without intermediate raw JSON copies. Collection
groups contain borrowed references, and only emitted change payloads become owned
JSON. The public borrowed comparison API still leaves its inputs intact, clones
only graphs when necessary, and borrows opaque artifact extensions throughout.

Every complete parsed CLI envelope in both retained cases matched the pre-change
response: no fields were excluded from equality, including digests, comparison
binding, freshness, record ordering and summaries. Core regressions additionally
cover singleton/duplicate function and file groups, unordered collection ordering,
ordered flow and opaque extension arrays, large unchanged opaque extensions and
invalid artifacts. Existing Git, pagination, byte-budget, session and MCP
acceptance remain required.

Independent [old/new parity evidence](memory-parity.json) covers 12 temporary
real-Git cases: unchanged data, metadata, edge variants, flow diagnostics, ordered
flow, duplicate inventories, opaque/node fields, unordered collections and invalid
schema/reference/type/provenance inputs. Complete query envelopes on every page
and legacy stdout, stderr and exit codes match. Graph files remain byte-identical.

The remaining roughly 2.07 GiB peak includes the two expanded JSON trees and
validation/sorting transients. Compact typed flow storage was deliberately left
outside this optimization. No graph schema, query response or comparison format
version changed.

### Supplemental upstream test result

The separate CallAtlas `dotnet test --configuration Release` run built all three
projects for .NET 11 and reported 262 passed and 29 failed tests. Independent
source/TRX review classified all 29 first failure sites as unqualified symbol-ID
assertions and lookups, such as `IShape::Draw()` and
`X::F()::Inner#local`, while the recorded assembly-identity patch emits qualified
IDs. This task changes only the three project target frameworks within that
upstream patch; it does not change its analyzer logic or upstream assertions.
Twenty-six failures concern snapshot assembly qualification and three concern
delegate assembly qualification. No distinct .NET 11-specific assertion failure
was found. This classification does not establish whether later assertions would
pass after correcting IDs, and no .NET 10 control run was performed.
This supplemental suite is outside the configured workspace verification gate
and is not reported as passing. The complete log, TRX and extracted failures are
retained under
`target/verification/task58-memory-optimization/upstream-tests/` (the command log
is `target/verification/task58-memory-optimization/upstream-net11.log`).

## Memory optimization verification

The final run at `target/verification/task58-memory-final-rerun/` passed all
43 stages outside the macOS sandbox using the installed .NET SDK 11.0.100/runtime
11 and Graphviz 16.1.1. It passed 574 ordinary Rust tests, all 667 release tests
(zero ignored), 218 Python tests, 112 TypeScript tests, CPU/GPU and storage
benchmarks, app/engine packaging and signature checks, installed CLI acceptance,
independent MCP installation and integration journeys, CodeScene and cleanup.
The analyzer, generated .NET fixtures, build/package paths and release CI target
.NET 11. The upstream project edits are captured in the checked-in patch; its
previous patch is retained as a verified bootstrap migration checkpoint.

Earlier attempts are retained separately: the initial memory run failed preflight
against outdated version requirements, the first latest-toolchain run was
explicitly interrupted before simplifying the remaining .NET 10 fixtures, and
`task58-memory-final-full` passed 666 release tests but failed one fixture's
`dotnet restore` assertion. That assertion omitted subprocess output. An exact
standalone recreation restored successfully, so its original cause remains
unknown. The assertion now includes status/stdout/stderr, without changing restore
requirements or adding retries; both MediatR identity tests passed in the final
full rerun. Both private Maven regressions passed; registry DNS retries delayed
dependency installation in the earlier attempt.

Every affected source file was reviewed with CodeScene at 10.0; the independent
review found no remaining actionable concerns. The pre-commit safeguard passed
for all 17 eligible files, and the final full CodeScene stage passed. SonarMCP
remains inactive because `.sonar/config.json` is absent. The supplemental upstream
test failures above are reported separately and are not claimed as passing.

## Initial delivery verification

The full run uses the existing compiler fixture modules and private Maven
checkout, and snapshots the saved application workspace before benchmarking.
Evidence is under `target/verification/task58-full/`. An earlier preflight-only
attempt under `full-w5n2id84` failed because the required input environment
variables were unset; it did not establish acceptance results.

All 52 stages passed: 568 ordinary Rust tests, 661 release tests including all
ignored compiler/real-process acceptance and CPU/GPU benchmarks, 215 Python tests,
112 TypeScript tests, storage benchmark, app/engine packaging and signature,
installed CLI acceptance, independently installed MCP and integration journeys,
CodeScene and artifact cleanup. The private UI dependency installation emitted
registry DNS warnings and delayed both private builds; both builds and their
acceptance tests ultimately passed. No exception was needed.

Every affected source file was reviewed with CodeScene; findings were resolved
to 10.0. The pre-commit safeguard and full CodeScene stage passed without
regressions. SonarMCP is inactive because this checkout has no `.sonar/config.json`.
