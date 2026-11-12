# Anchored build picker — 2026-11-12

Find builds now opens a modal beside its button and rescans in the background.
The checkbox list scrolls vertically, selection applies immediately, and the
button's hover help lists selected compact build names. The sidebar no longer
reserves inline build-list space.

Picker rendering, desktop discovery lifecycle and shared metadata naming have
separate modules. BuildTarget fields, identity and saved-session schemas remain
unchanged. Rescans retain both selected and deselected known identities; new
identities use discovery defaults. Changing folders invalidates old targets and
stale discovery results. Discovery error cleanup preserves unrelated warnings.

Project names come from literal Maven, Gradle or unconditional C# metadata, with
folder/file-stem fallbacks. XML uses roxmltree already present in Cargo.lock.
Gradle scripts containing triple-quoted strings conservatively use folder names;
no build scripts or computed names are evaluated for display.

## Visual evidence

Native Rune GPU rendering, inspected in both appearances:

- [Dark, 1420×900](builds-dark-1420.png)
- [Light, 1050×680](builds-light-1050.png)

The captures show the list after scrolling. The header remains fixed, and the
panel stays aligned with the button. Runtime tests additionally resize to
600×360 and verify the panel and scroll viewport remain usable.

Reproduce captures on macOS with native Metal access:

```sh
CODECALLNAVIGATOR_BUILD_CAPTURES=evidence/ui/build-picker-2026-11-12 cargo test --locked -p codecallnavigator --bin codecallnavigator ui_tests::build_picker::scrollable_modal_fits_windows_and_leaves_explorer_space -- --nocapture
```

## Verification

Passed:

- `cargo check --locked -p codecallnavigator --all-targets`
- `cargo check --locked -p codecallnavigator-core --all-targets`
- `cargo test --locked -p codecallnavigator --all-targets` — 373 passed before the final additional discovery-error reset regression; that regression and the complete workspace suite passed afterward.
- `cargo test --locked -p codecallnavigator-core --all-targets` — 60 passed.
- `cargo test --workspace --all-targets` — final run: 494 passed, 84 ignored.
- Focused discovery lifecycle tests: 4 passed; picker Runtime tests: 8 passed.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`
- `python3.13 -m unittest discover -s tests -v` — 106 passed.
- `python3.13 tools/package.py` — release desktop/engine built, local app/ZIP packaged and signature verified. Packaging required sandbox escalation for its temporary Git index; captures required native Metal access.
- CodeScene file reviews: all modified source files scored 10.0. Final pre-commit safeguard passed, checked 15 eligible files, no issues found.
- Independent planning, exploration, implementation, static review and Runtime testing roles completed; review findings fixed and rechecked.
- `git diff --check`

Required full ignored-test gate did not pass:

- `cargo test --workspace --all-targets -- --include-ignored` failed in the desktop test binary. Three ignored canvas benchmarks require release mode (two also require an explicit copied profiling workspace); the engine forced-cancellation test reported a surviving detached child under the sandbox. It stopped before later test binaries.
- An expanded `--no-fail-fast` attempt passed 214 desktop tests but retained those four failures. It then reported five ASP.NET acceptance failures and stalled in the remaining test's `dotnet restore fixtures/mediatr-dispatch/MediatRDispatch.sln`. The expanded run and its restore process were terminated; it is incomplete, and no full analyzer-acceptance success is claimed.
- Sonar is inactive because this workspace has no `.sonar/config.json`.

TASK-46 retains the full-validation limitation. Changes are local; no commit,
push or pull request was created.

## Subsequent full verification

The earlier full-suite limitations above were resolved during TASK-47. The final
native full verification on 2026-11-12 passed all 578 release workspace tests,
including ignored compiler acceptance and all canvas benchmarks, plus packaging
and CodeScene. No waivers were needed. See
[the complete verification record](../../verification/2026-11-12/README.md).
