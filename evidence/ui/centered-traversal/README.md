# Centered traversal verification

TASK-23 replaces selection dimming with a centered caller/callee neighborhood and
progressive group → class → method exploration. Class boxes list filtered,
scrollable method signatures. Opening a class method graph preserves its internal
call hierarchy with external context above and below.

The six PNGs use explicit synthetic traversal scenes, production projection and
Graphviz, and native Metal rendering. Dark/light normal-size class and method
views were inspected. Minimum-size class views use the explicit Fit action; Fit
reserves space beneath the floating filters. Focus itself preserves zoom and
centers its anchor, so large trees can require panning or Fit.

Reproduce with the built development binary:

```sh
./target/debug/codegraph --scene class-tree --snap /tmp/class-tree.png --theme dark --size 1420x900 --scale 1 --at 2
./target/debug/codegraph --scene method-tree --snap /tmp/method-tree.png --theme light --size 1420x900 --scale 1 --at 2
```

Use the class scene at 1050x680 and activate Fit for minimum-window checks.
Screenshots require native Metal device access. Physical trackpad and VoiceOver
verification are not claimed.

Behavioral coverage lives in the native traversal and graph-fit suites and the
projection focus tests. It covers click timing, both directions, class box hops,
method rows and long-list scrolling, internal class ordering, overload identities,
cycles, pinned pagination at the canvas limit, layer tabs, history, legacy focus,
filter invalidation, camera input during layout, and wrapped filter overlays.

Validation logs accompany these captures. Python packaging provenance tests and
real packaged Java resource smoke tests pass. A fresh app bundle cannot be built:
the existing ignored vendor/rune checkout has local changes rejected by the
packager's source-provenance guard. Those dependency changes were preserved.
