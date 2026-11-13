# Vendor source refresh — TASK-64

## Source selection and integration

Fetched the configured upstream repositories on 2026-11-13. Rune `dev` advanced
from `bba30a61cfcec6683e3fdedf156a92dae89d8b04` to
`c4eda05889c984fff91ccd906e3c00cdfc4cae24` (338 non-merge commits).
CallAtlas `main` remains `916c2419c0cdb91b5b86b84df4842dadb17bb6b4`;
java-callgraph2 `main` remains `77f566d0878958bab929b4cdeafcc2c762ff555e`.
Downloaded investigation binaries were outside this Git-source refresh.

Verified the complete original vendor diffs against the checksum-pinned patches.
Reversed only the exact old Rune patch, checked the checkout was clean, moved to
the new revision, and installed the rebased patch. No reset or stash was used.
All four Rune patch alternatives were rebased and their checksums refreshed;
they remain alternative combined patches, never patches to stack.

Observer wiring preserves the new `on_rest`, browser CPU/GPU startup, IME and
direction-chord behavior. Pointer observers precede autoscroll early returns.
The obsolete focus-method warning hunk was removed. The oriented-segment GPU
regression uses the updated metallib API; bytecode fingerprints were not changed
to match stale libraries. Runtime shader compilation remains intentional.

The consuming workspace now enables Rune's `tooling` feature so release
headless acceptance and packaged captures retain their behavior. It selects
Rune's upstream-patched wgpu-hal, winit, parley_engine and accesskit_windows
through workspace overrides; tiny-skia-path is already an upstream path dependency.
The root lockfile records these paths. No other root application API adaptation
was necessary for the all-targets compilation.

Rune's required integration changelog introduced an added file into the patch.
The migration fixture previously assumed every patch path existed upstream and
failed two setup cases. It now seeds pristine files from old-side paths and
uses the production whole-checkout comparison, including untracked added files.

## Verification

- Workspace all-targets `cargo check` passed.
- Seven Python dependency patch tests passed after the fixture correction.
- Independent pristine-source checks passed all four patch checksums and clean
  applications, checkpoint-to-current migration, repeat verification, and
  rejection/preservation of unrelated shader edits.
- Independent static review found no actionable rebase defects. Installed Rune
  HEAD matches the lock and its complete checkout diff matches the current patch.
- CodeScene reviewed the 18 touched Rust files in the rebased dependency and the
  migration fixture. The fixture scored 10.0; dependency findings concern upstream
  complexity, including existing input/startup functions. No broad upstream
  refactoring was introduced during this refresh.
- The focused upstream Rune test
  `oriented_segment_quads_preserve_caps_clips_and_layer_pixels` passed with native
  Metal access (1 passed, 2,184 filtered out). It checks pixel equivalence at 1x
  and 2x and rejection of stale precompiled shader bytecode.
- `./verify-full.sh --base origin/main` passed all 27 stages outside the macOS
  sandbox. Local logs and per-stage results are retained under
  `target/verification/full-e45ldvgw/`.
- Ordinary Rust: 674 passed, zero failed, 110 ignored; release acceptance:
  **784 passed, zero failed, zero ignored**. Both private Recurring Billing
  cases ran and passed. Storage and CPU/GPU benchmark stages passed.
- Python: 193 tests passed; tools suite ran 50 with one optional compiler
  journey skipped. That journey was then run explicitly with the real release
  CLI and TypeScript modules and passed (see `optional-freshness-journey.log`).
  All 112 TypeScript tests passed.
- Bootstrap, strict Clippy, package signatures, installed CLI, independent MCP
  installation, and real CLI/MCP compiler journeys passed. CodeScene's safeguard
  checked the changed project Python source and passed. Sonar is unconfigured.
- Packaged release headless captures completed in isolated temporary app stores
  in dark 1420×900 and light 1050×680 logical sizes. This also exercises the new
  explicit release `tooling` feature. See current [dark](vendor-refresh-2026-11-13/fixture-dark.png)
  and [light](vendor-refresh-2026-11-13/fixture-light.png) captures and the previous
  package's [dark](vendor-refresh-2026-11-13/baseline-dark.png) and
  [light](vendor-refresh-2026-11-13/baseline-light.png) captures. Visual inspection
  found the filter/card overlap already present in the previous package;
  this refresh does not fix that existing scene/layout limitation. The captures
  are not asserted to be pixel-identical.

Compilation emits upstream warnings in Rune and its nested tiny-skia-path
dependency. The standalone Rune test build also reports stale precompiled
shader libraries and other upstream test warnings. This is not a warning-free
upstream build. Shader freshness guards deliberately reject the bytecode for the
modified WGSL; upstream precompiled-library freshness tests are not claimed.

Browser CPU-first startup/handover is statically reviewed, not exercised in a
browser. Native tests and offscreen GPU measurements do not establish behavior
on Windows, Linux or web, nor native displayed-frame latency.
