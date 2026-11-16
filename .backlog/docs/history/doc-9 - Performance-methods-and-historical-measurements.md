---
id: doc-9
title: Performance methods and historical measurements
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:06:00Z'
---
# Performance methods and historical measurements


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-performance-analysis-md"></a>

## evidence/performance/analysis.md

Source: [evidence/performance/analysis.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis.md). Historical record; retained rationale, methods and limitations.

### Analysis performance

This work preserves the compiler-backed engines, source-set coverage, ordered
classpath selection, declaration policy, diagnostics, content verification and
atomic generation publication. Reuse is limited to the current analysis.

#### Changes

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

#### Measurement protocol

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

#### Measured results — 2026-11-09

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

Detailed medians, stages and work counts: [summary](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis-summary.json).
Raw five-run records: [baseline](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis-baseline.json), [optimized](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis-optimized.json).

#### Validation

The independent real-toolchain Rust run passed 187 tests; final focused checks also
passed the additional cross-build cache regression and completed-metrics behavior.
Two private recurring-billing cases were filtered because `CODEGRAPH_RECURRING_BILLING` and
`CODEGRAPH_COMPILED_METADATA` are unset. This includes C#, Java, TypeScript,
Gradle/Maven builds, duplicate/classpath order, generated sources, Lombok, lambdas,
packaged analyzer resources, UI behavior, cancellation and all 32 filter choices.
The Node compiler/worker/observation suites passed 30 tests. Python and quality
results are recorded in [test evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis-tests.json) and
[CodeScene evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/analysis-code-health.json).

Review found and corrected two regressions before final measurement: discarded
TSV-header errors and unbounded retained TypeScript workers. The corresponding
regressions reproduce malformed headers and concurrent real Node workers.
CodeScene does not support the `.gradle` init script; actual Gradle acceptance
verifies it. Sonar is inactive because this project has no `.sonar/config.json`.


<a id="source-evidence-performance-canvas-readme-md"></a>

## evidence/performance/canvas/README.md

Source: [evidence/performance/canvas/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/canvas/README.md). Historical record; retained rationale, methods and limitations.

### Dense canvas rendering

Measured on 2026-11-10 using the release build and the production Rune view on
macOS arm64. The deterministic fixture contains 300 visible cards and 6,000
straight, nearly straight and bowed cubic routes; one quarter are dashed dispatch
edges. Each zoom level has 12 warmup frames and 60 measured frames with camera
panning and forced view invalidation. Node count and UI layout work stay the same.

| Zoom | Before median / p95 (ms) | After median / p95 (ms) | Line primitives before → after |
| --- | --- | --- | --- |
| 0.16 | 5.477 / 6.176 | 3.283 / 3.739 | 159,518 → 46,101 |
| 0.20 | 6.098 / 6.675 | 3.662 / 4.286 | 166,484 → 59,620 |
| 0.24 | 6.457 / 7.142 | 4.344 / 5.204 | 174,463 → 69,697 |

Median CPU frame preparation improved by 33–40%, with 60–71% fewer line
primitives. These measurements exclude GPU presentation and do not establish
interactive display FPS. This synthetic workload isolates dense connector
rendering; timings depend on hardware and graph geometry.

Previously every cubic used 24 line segments, including straight or subpixel
curves. The shared graph and flowchart renderer now subdivides only when the
interior controls exceed 0.5 logical screen units from the finite chord. Testing
distance to a finite segment preserves collinear curves that double back.
Subdivision is capped at depth 12 (4,096 segments per cubic). At that cap, the
renderer prioritizes bounded work over the tolerance for pathological geometry.
Arrow direction uses the original endpoint tangent, independent of zoom and
tessellation. Straight connectors also avoid an extra polyline clone.

Reproduce on the baseline and modified renderer using the same benchmark:

```sh
cargo test --release --bin codegraph dense_canvas_frame_preparation -- --ignored --nocapture --test-threads=1
cargo test --release --bin codegraph edges::tests
cargo test --release --bin codegraph
```

Raw measurements are in [before.txt](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/canvas/before.txt) and [after.txt](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/canvas/after.txt).
The straight-cubic regression first failed against the original renderer (25
points instead of 2). Geometry coverage includes sampled curve error, loops,
inflections, backtracking, repeated controls, cubic joins, zoom-dependent detail,
endpoint direction, and bounded pathological input. Runtime tests cover existing
selection, fit, zoom, navigation and flowchart behavior.

Final validation: all 125 release binary tests passed (the manual benchmark is
ignored by default); release build, packaging and strict ad hoc signature
verification passed. CodeScene reviews scored 10.0 for all modified source files.
The parallel suite exposed a test that mutated selection without invalidating
the runtime; using `Runtime::update` makes its redraw explicit. Both serial and
parallel suites pass. No upstream source changes were needed. Sonar is inactive
because this project has no `.sonar/config.json`.

Native Metal captures at 1420×900, dark appearance and scale 1 were visually
checked for graph connectors, arrows and flowchart branches:
[call graph](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/canvas/fixture.png), [flowchart](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/canvas/flow.png).

```sh
./target/release/codegraph --scene fixture --snap /tmp/codegraph-canvas-fixture.png --theme dark --size 1420x900 --scale 1 --at 2
./target/release/codegraph --scene function-flow --snap /tmp/codegraph-canvas-flow.png --theme dark --size 1420x900 --scale 1 --at 2
python3.13 tools/package.py
codesign --verify --deep --strict dist/CodeGraph.app
```


<a id="source-evidence-performance-panning-readme-md"></a>

## evidence/performance/panning/README.md

Source: [evidence/performance/panning/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/panning/README.md). Historical record; retained rationale, methods and limitations.

### Full-detail graph panning

TASK-28 addresses work after TASK-26. Connector geometry, styles, labels, arrows,
and selection remain fully rendered during a pan. Session persistence keeps its
existing 300 ms debounce.

#### What changed

- Prepare zoom-scaled flattened routes once per installed layout and zoom; pan only
  translates them. Explicit layout replacement clears geometry and emphasis caches.
- Clip individual connector pieces against a padded viewport. Jump over hidden
  dash intervals analytically while retaining the original 7-on/4-off phase.
- Rasterize Rune line segments as oriented strips around their endpoint chord.
  The fragment coverage, caps, antialiasing, painter order and instance ABI are unchanged.
- Store the upstream shader and differential GPU tests as an exact pinned patch,
  applied and verified by dependency bootstrap and packaging. See patches/README.md.

#### Measurements

CPU preparation excludes GPU drawing and presentation. The saved-view pointer
benchmark emits 4,930 line segments versus 14,619 previously, with geometry built
only once during the drag. CPU results at 1x/2x are in before.txt and after.txt.
The original CPU baseline uses pre-change 16c80ca application modules in an
isolated temporary harness, with ordinary release feature flags; the dense
fixture and pointer motion are held constant.

GPU-complete measurements include CPU preparation, uploads, encoding, submission,
and waiting for completion, offscreen at 1900 x 1100 logical points. They use
10 warmup and 30 measured pan frames and are **not displayed FPS**.

| Workload | Scale | Before p95 (ms) | Oriented quads p95 (ms) |
| --- | --- | --- | --- |
| Dense: 300 cards / 6,000 edges | 1x | 27.52 | 12.91 |
| Dense | 2x | 51.33 | 12.92 |
| Saved: 31 cards / 155 relationships | 1x | 19.44 | 4.78 |
| Saved | 2x | 10.93 | 7.40 |

The GPU before/after comparison already includes app-side clipping/caching in
both runs and holds the primitive counts constant. Dense 2x median drops from
47.43 ms to 12.45 ms. Short saved-view timings include scheduling variation; the
results do not imply that 2x is intrinsically faster than 1x. Raw outputs are in
gpu-before.txt and gpu-after.txt. Hardware: Apple M4 Pro.

#### Reproduction

Run from the repository root, using a copied workspace for saved-state benchmarks:

```sh
CODEGRAPH_DATA_DIR=/absolute/copied/store cargo test --offline --release --bin codegraph canvas_frame_preparation -- --ignored --nocapture --test-threads=1
CODEGRAPH_DATA_DIR=/absolute/copied/store cargo test --offline --release --bin codegraph canvas_gpu_frame_completion -- --ignored --nocapture --test-threads=1
CARGO_NET_OFFLINE=true python3 tools/package.py
python3 tools/pan_profile.py run --binary dist/CodeGraph.app/Contents/MacOS/CodeGraph --output /absolute/new-results --scene saved-canvas --expected-scale 1
python3 tools/pan_profile.py run --binary dist/CodeGraph.app/Contents/MacOS/CodeGraph --output /absolute/another-new-results --scene dense-canvas --expected-scale 1
```

The native runner snapshots the catalog transactionally and copies its immutable
active generation. It never intentionally writes to the live workspace. If a
read-only WAL snapshot cannot open under a restricted environment, provide an
already-consistent copied store with --data-dir rather than bypassing snapshot
consistency. The saved profiling scene requires existing native repository layer
configuration and does not import or migrate files.

#### Native results

The signed packaged app passed all three ten-second gestures in both scenes on
the connected 120 Hz display. Actual active window dimensions were 1147 x 1344
logical points at 1x backing (the startup request was 1900 x 1100). The reporter
uses live dimensions; these native results are separate from the fixed-size
offscreen tests above.

| Native workload | Distinct camera presentations/s | Worst gesture p95 interval |
| --- | --- | --- |
| Saved workspace | 119.29–119.59 | 8.334 ms |
| Dense fixture | 96.59–101.51 | 16.668 ms |

Both exceed 30 fps and fit the nominal 33.3 ms presentation budget. These are
synthetic pointer events through production drag handlers, paired with actual
Metal presentation timestamps. OS mouse-delivery latency is outside this probe.

`native-saved/` and `native-dense/` contain the run identities, summaries, CPU
reports and reduced native traces. Reduction retains the exact timing fields
and codegraph.* channel values needed to recompute acceptance; UI element
metadata and geometry are omitted. Each trace-provenance.json records the
original full trace hash. Re-evaluate the retained evidence with:

```sh
python3 tools/pan_profile.py report evidence/performance/panning/native-saved --expected-scale 1
python3 tools/pan_profile.py report evidence/performance/panning/native-dense --expected-scale 1
```

The measured signed executable SHA-256 is
`353d491d204acf3060043b570354fff0326eca12a8c829acc30144b9fe025afc`.
Code signing changes executable bytes, so the packaged hash is recorded directly.
A final repeat using the checked-in GPU benchmark remains below budget in every
case; its dense 2x p95 is 14.25 ms (gpu-final.txt). CPU saved-view p95 is 1.42 ms
at both scales (after.txt). Short benchmark runs have scheduling variation.

#### Verification and limitations

141 release application tests pass; three manual performance benchmarks are
ignored by default and were run separately. The GPU differential regression
passes at 1x/2x against the original AABB shader with at most one RGBA8 LSB of
pixel difference, covering caps, slopes, degenerate segments, fades, clipping,
transparency and isolated layer projections. See pixels.txt. It was built from
byte-identical patched source in a minimal offline Rune workspace: the full
upstream workspace requires unavailable unrelated dependencies, and its release
unit-test configuration has pre-existing debug-only field references. Reproduce
that focused test in a dependency-complete checkout with:

```sh
cargo test --manifest-path vendor/rune/crates/rune/Cargo.toml --lib oriented_segment_quads_preserve_caps_clips_and_layer_pixels -- --nocapture --test-threads=1
```

Twelve native-report tests and two dependency-patch migration tests pass.
All modified application/tool source files score CodeScene 10.0; the pre-commit
gate passes. The upstream Rust test file retains its previous 7.83 score and
findings. CodeScene does not support WGSL; GPU compilation, differential pixels
and throughput verify the shader. Sonar is inactive because .sonar/config.json
is absent. Release packaging, exact dependency source verification and strict
deep signature verification pass. Saved/dense/flow GPU captures were inspected.

The opt-in compositor queues three ten-second horizontal/diagonal sweeps after
the current view settles. The normal application installs no compositor or
profiling clocks. Native --script does not replay gestures and is not used.

Acceptance counts distinct camera positions paired with actual Metal present_ms,
requires full gesture coverage, measures FPS over actual presentation duration,
and requires p95 intervals within 1000/30 ms plus trace rounding. Headless clocks,
idle cameras, missing presents, long stalls and changed binaries cannot pass.
Active runtime scale and pixel/point ratios are checked independently of the
startup header.

GPU query readback is nonblocking and bounded. Metal returned invalid zero or
reversed timestamp pairs on some frames; those are excluded, counted and
reported. The remaining native query samples do not establish a whole-run GPU
duration distribution. The independent GPU-complete benchmark waits for all
work and therefore establishes the offscreen rendering cost. Exact drawable
acquisition waits and OS input arrival require external platform tracing.

The only connected display has native 1x backing. Native 2x display cadence
requires a 2x display; --scale cannot change native backing. Offscreen GPU-complete
throughput, pixels and CPU/runtime behavior pass at both scales.

The sandbox initially hid Metal. Graphics access outside the sandbox succeeded.
Later native retries stopped while macOS was locked; those runs were rejected.
After the user unlocked the desktop, both final packaged-app native runs passed.
One optional old-renderer native comparison timed out with display-link error
-6661 and is not used as FPS evidence.

#### Local source commits

- 7cb1302: viewport clipping, analytic dash skipping and pan-independent caches.
- 09c42ac: measured oriented Rune quads, pinned patch and safe bootstrap migration.
- 473fb65: native profiler, saved/dense scenes and reproducible frame benchmarks.

No push or pull request was performed. The Backlog TASK-28 record contains the
chronological implementation/review/testing notes and acceptance evidence.
