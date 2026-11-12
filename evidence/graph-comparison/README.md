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

Memory use is substantial on this corpus: the first version retains both parsed
artifacts and canonical comparison copies. Pages are stateless and re-read and
compare their endpoints; a small response does not imply a small working set.

## Full verification

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
