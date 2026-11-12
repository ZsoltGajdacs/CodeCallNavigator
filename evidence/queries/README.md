# CLI query measurements

Task: TASK-48. Measurements taken on 12 November 2026 with an optimized engine,
using the existing 113,628,316-byte compiler-produced Java project artifact.
The measured project and its artifact were read only. No live desktop store was
benchmarked. Raw durations and graph digests are in `billing-latency.json`.

| Operation | One-shot median | Persistent session median |
| --- | ---: | ---: |
| Status | 1,860 ms | 233 ms |
| Search | 1,864 ms | 234 ms |
| Overview selection | 1,891 ms | 249 ms |
| Complete render snapshot | 4,425 ms | 3,275 ms |

Each operation has three samples. Timings include process work, pipe transfer
and Python JSON decoding. Filesystem caches were uncontrolled, so these are
not cold-disk benchmarks. The first session status took 1,945 ms to load the
generation; subsequent status requests took 233 and 230 ms. Full snapshots
transfer all analyzed data and are intended for generation changes rather than
interactive queries. Session reuse reduces search and selection latency by
approximately eightfold on this graph, but does not promise sub-100 ms results.

The CLI session hashes the current artifact bytes for every request, rejects a
different expected generation, and checks observed source inputs again even
when reusing the parsed graph. Artifact identity is not inferred from timestamps.

Reproduce against a prepared project without running analysis:

```sh
cargo build --locked --release -p codecallnavigator-engine --bin codecallnavigator-engine
python3 tools/query_benchmark.py --engine target/release/codecallnavigator-engine \
  --project /path/to/prepared/project --output /tmp/query-latency.json --repeats 3
```

Behavioral regressions cover session freshness after source edits, generation
replacement, generation-bound rejection, and orderly EOF shutdown in
`crates/codecallnavigator-engine/tests/query_session.rs`. Existing verifier
integration tests continue to cover mixed stale/unverifiable inputs and
read-only behavior.

## Task 48 verification

The full run at `target/verification/task48-full-20261112/` passed all runtime
stages: 502 ordinary Rust tests, 587 release-mode Rust tests including ignored
acceptance/benchmarks, 155 project Python tests, 26 tool tests and 112 TypeScript
tests; storage benchmarking, packaging and strict signatures also passed.

The original final quality stage failed because declaration-only Rust modules
returned a null CodeScene score. The runner now records their explicit
non-applicability using a conservative declaration-only grammar; executable and
unknown source still require a numeric score. This tooling-only correction
passed independent review, 16 quality tests and 19 orchestrator tests. A test
fixture builder was also split by responsibility after a Large Method finding;
its six query tests and Clippy passed again. Production code was unchanged after
the full runtime run.

The affected quality stage was rerun over every current changed source:
`35-codescene-rerun.json` reports success and 19/19 eligible files checked with no
regressions. The original failed report is preserved alongside the recovery
report rather than overwritten. No verification exceptions were accepted.
