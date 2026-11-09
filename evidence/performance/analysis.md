# Analysis performance

This work preserves the compiler-backed engines, source-set coverage, ordered
classpath selection, declaration policy, diagnostics, content verification and
atomic generation publication. Reuse is limited to the current analysis.

## Changes

- Gradle still discovers ownership before compilation, then compiles and exports
  final ownership in one invocation. The existing no-daemon policy remains.
- Java staging indexes classpath artifacts into job-local manifests. Dependency
  byte arrays are discarded after indexing; project class bytes are retained once
  for context staging. Each context independently applies its original ordered
  classpath and emits the same shadow diagnostics. Bytecode engines stay sequential.
- Artifact preparation uses two workers and a two-artifact window. Ordered
  selection and diagnostics stay on one thread; duplicate inputs share a lock.
  Artifact-index files use buffered reads and writes, avoiding native I/O for
  each serialized field. Parsed source inventories are shared across contexts. Normalization streams
  validated rows, caches class/source facts and uses typed full-field comparisons.
  Graph accumulators retain identity indexes through composition.
- Source traversal deduplicates overlapping roots only under the same exclusions.
  Separate phase snapshots still hash contents, missing paths and directory names.
- A four-entry cache per immutable graph retains cycle membership and hidden counts
  under all five filter flags. Searches use a scalar persisted policy check; older
  generations retain a typed fallback. Search ordering and pagination are unchanged.
- A single-target TypeScript refresh retains its compiler worker. Multi-target and
  mixed refreshes release preparation workers immediately and recreate one at a
  time, checking complete metadata equality and original snapshots. The worker
  verifies consumed contents, missing probes, directory queries and realpaths before
  and after extraction. Cancellation and abandoned jobs terminate and reap workers.

## Measurement protocol

Both executables are release builds with identical pinned engines. The baseline
executable and its complete adapters were frozen before source edits. Five runs
per workload use separate CodeGraph SQLite caches, existing toolchain/dependency
caches, and normal incremental build behavior. These are warm-dependency analyses,
not dependency-download or clean-build measurements. First-run and repeat-run
results are retained separately in the JSON evidence. No application tests run
inside an analysis.

`tools/measure_analysis.py` records wall time, raw macOS `time -l` resources,
sampled process-tree RSS, sampled job-temporary disk use, and optional stage/work
counters. Sampling is every 250 ms and can miss brief peaks. RSS sums can count
shared pages more than once; they are comparable process-tree observations, not
physical-memory accounting. Temporary disk excludes build outputs, dependency
caches and published graph databases. Timing stages can nest, so their durations
must not be added. The CLI does not build an in-memory GraphIndex or render a
layout; the native app measures indexing, publication and its initial layout
using the same collector.

Example:

```sh
python3.13 tools/measure_analysis.py \
  --executable target/release/codegraph \
  --workload storyengine=/path/to/StoryEngine/backend \
  --runs 5 --timings --output /tmp/codegraph-optimized \
  --baseline /tmp/codegraph-baseline
```

The harness compares every persisted node field; caller/target identities; edge
kind, certainty, provenance and count; entries; diagnostics; roots; engine versions;
method policy; and normalized freshness inputs. Node/edge storage offsets and
fingerprint observation timestamps are ignored. The baseline itself regenerates
164 StoryEngine source files with changing OpenAPI/MapStruct `@Generated` dates.
The harness derives an explicit hash-volatility set only from generated files
whose hashes already vary across baseline runs while sizes and all other facts
stay fixed. It reports those hash differences separately. Every input path, missing
state, exclusion and size remains strict; all other content hashes remain strict.
This normalization applies only to comparisons between separate builds. Runtime
snapshots still verify every complete content hash, including generated sources.
A mismatch fails the harness even when node and edge counts match.

## Measured results — 2026-11-09

Five-run medians on an Apple M4 Pro with 48 GiB RAM, with the resource sampler enabled:

| Workload | Baseline | Optimized | Change |
|---|---:|---:|---:|
| storyengine | 33.547 s | 27.150 s | 19.1% faster |
| gradle | 8.300 s | 5.942 s | 28.4% faster |
| maven | 5.213 s | 5.085 s | 2.5% faster |
| csharp | 2.660 s | 2.739 s | 3.0% slower |
| typescript | 0.703 s | 0.440 s | 37.3% faster |

All 25 optimized runs succeeded and matched the baseline graph semantics and
freshness-input coverage under the documented generated-hash policy. StoryEngine
exceeds the 15% target; no other workload regressed by more than 5%.

| Workload | Median sampled peak RSS, baseline → optimized | Max sampled temporary disk, baseline → optimized |
|---|---:|---:|
| storyengine | 907.8 → 961.8 MiB | 56.5 → 71.7 MiB |
| gradle | 615.3 → 608.3 MiB | 0.1 → 0.1 MiB |
| maven | 264.9 → 254.7 MiB | 0.1 → 0.2 MiB |
| csharp | 269.5 → 264.5 MiB | 0.0 → 0.0 MiB |
| typescript | 217.6 → 227.7 MiB | 0.0 → 0.0 MiB |

StoryEngine uses about 54 MiB more median sampled peak RSS (about 6%) in this
measurement. Artifact preparation is limited to two in-flight artifacts; manifests
and project bytes are scoped to the job and removed on success. Temporary disk
increases because those immutable intermediates are retained across contexts.
The RSS observations include build/compiler variability and are not a precise
allocation attribution. No persistent analysis cache was added.

Detailed medians, stages and work counts: [summary](analysis-summary.json).
Raw five-run records: [baseline](analysis-baseline.json), [optimized](analysis-optimized.json).

## Validation

The independent real-toolchain Rust run passed 187 tests; final focused checks also
passed the additional cross-build cache regression and completed-metrics behavior.
Two private recurring-billing cases were filtered because `CODEGRAPH_RECURRING_BILLING` and
`CODEGRAPH_COMPILED_METADATA` are unset. This includes C#, Java, TypeScript,
Gradle/Maven builds, duplicate/classpath order, generated sources, Lombok, lambdas,
packaged analyzer resources, UI behavior, cancellation and all 32 filter choices.
The Node compiler/worker/observation suites passed 30 tests. Python and quality
results are recorded in [test evidence](analysis-tests.json) and
[CodeScene evidence](analysis-code-health.json).

Review found and corrected two regressions before final measurement: discarded
TSV-header errors and unbounded retained TypeScript workers. The corresponding
regressions reproduce malformed headers and concurrent real Node workers.
CodeScene does not support the `.gradle` init script; actual Gradle acceptance
verifies it. Sonar is inactive because this project has no `.sonar/config.json`.
