# Uniform focused graph levels

TASK-29 follows TASK-23. Focused projection resolves one semantic presentation
level before aggregating and traversing relationships. Class, namespace/package,
folder, project, layer and method selections therefore align connected boxes,
independently of manual overview expansion. Folder connections use the selected
build-relative directory depth, with shallower and project-root fallbacks.

The `aligned-classes` scene deliberately expands only the service folder and
marks a caller class as expanded while the other folders remain collapsed. Both
native Metal captures were inspected: OrderController, OrderService and
OrderRepository are class boxes, connected by two edges. The unrelated Metrics
class is absent. The selected service retains its scrollable method list. Dark
and light themes show readable labels, directed edges and the selected anchor.

Reproduce after `cargo build --offline`:

```sh
./target/debug/codegraph --scene aligned-classes --snap evidence/ui/aligned-levels/classes-dark.png --theme dark --size 1420x900 --scale 1 --at 2
./target/debug/codegraph --scene aligned-classes --snap evidence/ui/aligned-levels/classes-light.png --theme light --size 1420x900 --scale 1 --at 2
```

Native capture requires Metal device access; the restricted sandbox lacks that
device, so captures were run with the normal native-device permissions.

Verification:

- The initial class/folder regression failed before the fix and passes afterward.
- `cargo test --all-targets --offline`: 301 passed, 50 ignored, no failures.
  See `rust-tests.log`. The ignored tests require explicit toolchains, fixtures or
  benchmark configuration; they are not counted as passes.
- Projection tests cover complete package/global namespace identities, folder
  nesting and root fallbacks, connected projects, layer-qualified identities and
  active tabs, filters, method focus, explicit child focus, cycles and pagination.
- Native interaction tests cover folder/package/class drill-down, persisted child
  focus, Back/Forward, unchanged overview expansion, depth and camera behavior.
- Production Graphviz layout checks and `cargo build --offline` passed.
- Scoped Rust formatting and `git diff --check` passed.
- Strict Clippy reports existing `needless_range_loop` in `src/edges/viewport.rs`
  and `assertions_on_constants` in the canvas benchmark tests. These files match
  HEAD and were left unchanged. Clippy passes with only those two lint classes
  exempted; see `clippy-exemptions.log`. This is not a clean strict-Clippy result.
- CodeScene reviewed all 11 changed source files at 10.0 with no remaining
  findings. Test fixture extraction and a combined navigation assertion resolved
  the initial test maintainability findings. The safeguard passed all 11 eligible
  files with no regressions.
- Independent production review found no blockers; requested project and layer
  coverage was added. Independent final testing confirmed the full Rust suite.
- Sonar is inactive because this checkout has no `.sonar/config.json`.
- Dependency source provenance verification passed. No upstream sources changed.
- Both packaged Java resource smoke tests passed with real Maven/Gradle/JDK
  artifacts, including rejection of a missing packaged extension; see
  `packaged-java.log`. The first restricted attempt could not access Gradle's
  cache lock; rerunning with normal toolchain cache access passed.
- Python acceptance/packaging unit checks passed: 50 tests via
  `python3.13 -m unittest discover -s tests -p 'test_*.py'`.

The development binary was rebuilt. No release bundle was created or installed,
and no commit or push was performed.

Single-click keeps a group at its own level. Double-click now persists explicit
child-group focus rather than modifying overview expansion. Existing group focus
sessions consequently restore at the group's own level; existing method focus
and class-method tree variants remain supported. No analyzer graph migration is
required. Physical trackpad and VoiceOver checks were not performed.
