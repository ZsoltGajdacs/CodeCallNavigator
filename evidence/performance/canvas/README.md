# Dense canvas rendering

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

Raw measurements are in [before.txt](before.txt) and [after.txt](after.txt).
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
[call graph](fixture.png), [flowchart](flow.png).

```sh
./target/release/codegraph --scene fixture --snap /tmp/codegraph-canvas-fixture.png --theme dark --size 1420x900 --scale 1 --at 2
./target/release/codegraph --scene function-flow --snap /tmp/codegraph-canvas-flow.png --theme dark --size 1420x900 --scale 1 --at 2
python3.13 tools/package.py
codesign --verify --deep --strict dist/CodeGraph.app
```
