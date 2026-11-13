# CLI-owned saved-input observations (TASK-60)

The production `freshness` query and `observe` stream use the same core manifest
verification pass. The query executor owns saved artifact/index reuse; observation
does not construct the graph index. The stream owns sequential full reconciliation
with a nominal 250 ms delay between passes and a five-second heartbeat. Its bounded
output coalesces current state rather than providing a durable event journal.

This is explicitly limited `verified_polling`, without native event acceleration.
Every emitted status comes from verification. Generation identifies immutable
saved graph bytes; input revision identifies current observed contents and scope
membership, including later edits to newly added files. Volatile observations are
not written into canonical artifacts. See the [wire contract](../../../docs/cli-queries.md).

## Measured fixture results

Each route ran all nine existing scenarios: body, signature, cross-file, addition,
deletion, rename, atomic save, burst and configuration. Each scenario has one first
sample followed by 20 warm repetitions. All **756 records** passed final-revision,
generation, sequence, receipt-time and shutdown checks. All 36 scenarios met the
one-second p95 research objective on these small fixtures.

| Compiler route | Warm samples | Worst scenario p95 | Body fixture nodes / observed inputs |
| --- | ---: | ---: | ---: |
| TypeScript | 180 | 262.6 ms | 5 / 91 |
| C# | 180 | 264.0 ms | 3 / 8 |
| Java/Maven | 180 | 264.9 ms | 3 / 21 |
| Java/Gradle | 180 | 264.7 ms | 3 / 22 |

Fixtures contain two authored source files, or three for deletion/rename baselines.
This measures save-start to receipt of a complete verified stream line, including
the saved-write duration. The final edited revision is established independently
through a one-shot query before starting the stream. Each repetition consumes a
later edited event and then a later restored-baseline event. First samples are
reported separately; they are not cold dependency or compiler starts.

All recorded scopes were completely observed in this run. TypeScript, C# and Maven
baselines remain **unverifiable** because inputs outside declared portable roots
remain explicitly `unmapped_input`; Gradle baselines matched. Complete scope
coverage does not certify the build environment. No larger-project latency target
is claimed: all modes remain explicitly limited, and scan duration adds to the
nominal polling delay. Unrecorded inputs, unsaved buffers and nested directory
links outside explicit scopes retain the documented limits.

`fixture-series/samples.jsonl.gz` contains the losslessly compressed raw records.
`summary.json` retains per-scenario distributions, baseline analysis metrics,
artifact/source/input sizes, reference observations and process-tree memory/CPU
reports. `metadata.json` records the executable/analyzer hashes, toolchains,
hardware and exact command. CPU totals include startup and resource-sampler child
processes; they are not isolated steady-state observer CPU measurements.

Reproduce with installed analyzer resources and a new output directory:

```sh
cargo build --locked --release -p codecallnavigator-engine --bin ccnavigator
python3.13 tools/measure_freshness_observer.py \
  --executable target/release/ccnavigator \
  --typescript-modules "$CODECALLNAVIGATOR_TS_NODE_MODULES" \
  --runs 20 --output /tmp/ccn-observer-evidence-new
```

## Correctness and review

Core regressions and independent probes cover stale-to-stale repairs, nested new
files, missing inputs, linked roots, per-scope exclusions, explicit generated
inputs, Maven output markers, directory and file symlinks, broken-link retargets,
device/FIFO rejection and active hashing cancellation. Unix input hashing uses
nonblocking opens and validates the opened descriptor before reading regular
files, with cancellation checks per chunk.

Real CLI/compiler journeys cover generation replacement and bound-generation
rejection, malformed-artifact failure with explicit restore/restart, capability
negotiation, legacy one-shot/session behavior, heartbeat, EOF and broken stdout.
An undrained full stdout pipe does not prevent EOF shutdown. Independent review
also removed an unbounded acknowledgement queue before measurement collection.

Core release tests: 89 passed. Independent CLI checks: six freshness, seven
legacy query (including real TypeScript) and two session tests passed. The eleven
measurement/compiler/process regressions passed. Modified eligible source files
passed CodeScene review and the pre-commit safeguard (24 files, no regressions).
Full workspace verification passed all **50 stages**, including the complete
release suite, benchmarks, packaging/signatures, installed CLI and independent
MCP journeys. See [verification/results.json](verification/results.json) and the
adjacent quality/install logs. The initial formatting-only failure is retained;
the successful retry used scoped Rust formatting and `--timeout 3600`.
