# Full-detail graph panning

TASK-28 addresses work after TASK-26. Connector geometry, styles, labels, arrows,
and selection remain fully rendered during a pan. Session persistence keeps its
existing 300 ms debounce.

## What changed

- Prepare zoom-scaled flattened routes once per installed layout and zoom; pan only
  translates them. Explicit layout replacement clears geometry and emphasis caches.
- Clip individual connector pieces against a padded viewport. Jump over hidden
  dash intervals analytically while retaining the original 7-on/4-off phase.
- Rasterize Rune line segments as oriented strips around their endpoint chord.
  The fragment coverage, caps, antialiasing, painter order and instance ABI are unchanged.
- Store the upstream shader and differential GPU tests as an exact pinned patch,
  applied and verified by dependency bootstrap and packaging. See patches/README.md.

## Measurements

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

## Reproduction

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

## Native results

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

## Verification and limitations

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

## Local source commits

- 7cb1302: viewport clipping, analytic dash skipping and pan-independent caches.
- 09c42ac: measured oriented Rune quads, pinned patch and safe bootstrap migration.
- 473fb65: native profiler, saved/dense scenes and reproducible frame benchmarks.

No push or pull request was performed. The Backlog TASK-28 record contains the
chronological implementation/review/testing notes and acceptance evidence.
