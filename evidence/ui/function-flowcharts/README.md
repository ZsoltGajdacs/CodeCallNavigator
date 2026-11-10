# Function flowchart UI evidence

Four native Metal screenshots show the dedicated chart/source workspace in dark
and light appearance at 1420×900 and 1050×680. The selected call step highlights its
cached source and exposes a compiler-attributed target button.

The explicit `function-flow` scene uses a two-function synthetic snapshot with the
production builder and Graphviz layout. It does not substitute for real compiler
acceptance tests. Source paths intentionally do not exist on disk, proving that
this view reads the snapshot. The captures were visually inspected for source
alignment, readable branches, accessible controls and minimum-window bounds.

Reproduce after building the development binary:

```sh
./target/debug/codegraph --scene function-flow \
  --script evidence/ui/function-flowcharts/explore.json \
  --snap /tmp/function-flow.png --theme dark --size 1420x900 --scale 1 --at 2
```

Use `--theme light` and `--size 1050x680` for the remaining matrix entries.
Native Metal access is required. Physical trackpad and VoiceOver validation are
not claimed.

Runtime interaction tests cover the inspector action, actual target-button
navigation, source-row highlight movement, flow history/camera restoration,
unchanged call-graph navigation, per-visit source scrolling (including A → B → A),
generation invalidation, old-cache messaging and
minimum-window geometry. Tests found and fixed the initial fast-load fit race and
newly inserted footer controls being clipped by animated sizing.
