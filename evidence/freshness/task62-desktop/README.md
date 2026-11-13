# Automatic-refresh desktop acceptance

The focused release test passed on 2026-11-13 in 15.19 seconds using real
TypeScript compilation, the installed CLI process contract, Rune controls and
native Metal captures. Exact binary and image hashes are in [run.json](run.json).
This is a desktop journey, not a language-wide latency benchmark or a replacement
for full verification.

The journey proves passive observation does not analyze/adopt; explicit session
opt-in stays bound to the loaded canonical project despite a different pending
folder; publication uses generation-bound adoption and preserves the camera;
invalid code preserves the previous graph, a saved repair recovers; manual
analysis waits for automatic cleanup; and opting out returns to passive behavior.

## Reproduce

From the workspace root, with matching release CLI and analyzer resources built:

```sh
CODECALLNAVIGATOR_ENGINE="$PWD/target/release/codecallnavigator-engine" \
CODECALLNAVIGATOR_RESOURCES="$PWD" \
CODECALLNAVIGATOR_TS_NODE_MODULES=/absolute/path/to/installed/node_modules \
CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES="$PWD/target/task62-desktop" \
cargo test --release --locked -p codecallnavigator --bin codecallnavigator \
  ui_tests::cli_freshness::actual_cli_automatic_refresh_requires_opt_in_and_preserves_loaded_context \
  -- --ignored --exact --nocapture
```

Run outside the macOS sandbox for compiler-process inspection/cleanup and Metal.
The source tree and live application store are not used as editable inputs.

## Visual review

The captures were inspected. Opt-in and cancellation controls remain accessible;
freshness validity and refresh activity are separate. Failed compilation has a
concise label with full diagnostics available through its tooltip, including in
the compact light theme. Automatic adoption intentionally retains the test's
panned/zoomed camera, so these captures do not demonstrate an explicit Fit action.

- [Enabled, dark wide](automatic-enabled.png)
- [Invalid code, dark wide](automatic-invalid-code.png)
- [Invalid code, light compact](automatic-invalid-light-compact.png)
- [Disabled, dark wide](automatic-disabled.png)
