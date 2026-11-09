# Architectural layer UI evidence

The `architectural-layers` scene contains six explicit synthetic methods. It uses
production projection and Graphviz geometry, with Controller, Service, Repository
and Tests bands, a Utilities side lane, and the original caller-to-callee arrows
(including a cross-layer cycle). It does not claim analyzer acceptance for a real
repository.

The four screenshots cover dark/light appearance at 1420×900 and the minimum
1050×680 window. The capture script closes the side panes and fits the graph.
Visual inspection confirmed visible controls, ordered bands, side-lane placement,
unchanged edge categories and cycle styling. Pan, zoom and method focus retain
existing behavior; physical trackpad and VoiceOver validation are not claimed.

Reproduce after `cargo build --locked --bin codegraph`:

```sh
./target/debug/codegraph --scene architectural-layers \
  --script evidence/ui/architectural-layers/explore.json \
  --snap /tmp/codegraph-layers.png --theme dark --size 1420x900 --scale 1 --at 3
```

Use `--theme light` and/or `--size 1050x680` for the other matrix entries. Native
Metal device access is required for screenshots.

Runtime interaction tests additionally verify that presets and tabs work, history
restores the tab independently of the saved preset, and controls remain unclipped
with both side panes open at the minimum window size. An immediate post-transition
check prevents the graph canvas/filter overlay from covering freshly inserted tab
controls. Projection regressions verify real Graphviz ordering, independent
parallel flows, support lanes, cycles/reverse arrows, tab omission accounting,
filtered-edge counts and dotted Java build-root source paths. Workspace regressions
exercise cached-content reclassification, invalid-configuration recovery, disabling
layers and rejecting obsolete graph epochs.
