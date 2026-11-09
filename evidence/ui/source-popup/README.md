# Hover source viewer verification — TASK-15

Implemented the approved method-source and aggregate-file popup in the production
Rune view. Analyzer output, graph storage and CLI analysis contracts are unchanged.

## Behavior evidence

- Thirteen real Rune runtime tests in `src/ui_tests/source_popup.rs` exercise the
  actual graph cards, temporary source files, asynchronous jobs and production UI.
  They cover idle hover timing, leave grace, popup/trigger overlap, pin/unpin,
  Escape/Close, file listing and Back, target-line reveal, both scroll axes,
  stale/invalid-location messages, obsolete worker results, navigation dismissal,
  pane/window resizing, visible-size resize origins, clipboard copy without line
  numbers, long Unicode lines, and isolation from covered graph gestures.
- Source tests read real UTF-8/CRLF files, preserve trailing empty lines, enforce
  bounded reads and explicit failures, and test multiline Java/C#/TypeScript/TSX
  lexical highlighting beyond Kit's 32 KiB cap.
- Projection tests verify nested directory membership, namespaces, filtering,
  deduplication, identical project names in different builds, and membership
  exceeding the graph's 1,500-node page limit.

## Visual inspection

The explicit `source-preview` and `source-files` tooling scenes use actual
checked-in Java files with synthetic graph relationships. They never replace
normal startup. The screenshots come from the native Metal renderer.

| Capture | Appearance / window | Verified |
| --- | --- | --- |
| `source-dark.png` | Dark, 1420×900 | Syntax colors, location marker, path, controls and long unwrapped lines |
| `source-light-min.png` | Light, 1050×680 | Window clamping, readable code and reachable resize/close controls |
| `files-light.png` | Light, 1420×900 | Distinct readable relative source paths |
| `files-dark-min.png` | Dark, 1050×680 | Listing and controls fit the minimum window |
| `files-resized.png` | Dark, 1050×680 | Actual corner resizing and scrolling a listing longer than its viewport |

No actionable clipping or control-placement findings remain. A partially visible
first/last source line is normal scroll clipping; the path/header and footer stay
outside the scrolling viewport.

Reproduce, changing scene, theme, size and output as needed:

```sh
cargo build --locked --bin codegraph
./target/debug/codegraph --scene source-preview \
  --script evidence/ui/source-popup/hover.json \
  --snap /tmp/source-popup.png --theme dark --size 1420x900 --scale 1 --at 3
./target/debug/codegraph --scene source-files \
  --script evidence/ui/source-popup/resize-files.json \
  --snap /tmp/files-resized.png --theme dark --size 1050x680 --scale 1 --at 3
```

## Validation and action log

1. Recorded the approved plan and criteria through the Backlog CLI. Its task-lock
   operation required access to `.git`; retry with approved sandbox escalation
   succeeded.
2. Observed the projection membership test fail on absent metadata, then added
   typed groups and file membership before pagination.
3. Implemented bounded background reads and isolated lexical highlighting. Source
   tests passed using actual files; syntax regressions were reproduced before fixes.
4. Connected transient state, absolute hover deadlines, root-layer rendering,
   virtualized code/list rows, line reveal, pinning and corner resizing.
5. Review found blank-area input pass-through, covered-node hover replacement and
   late deadline initialization. Fixed these and verified through runtime tests.
6. Further review fixed partial trigger overlap, visible-size resize origins,
   virtual-row key reuse, TSX prose quotes, constrained/defaulted generic arrows,
   and escaped Java text-block delimiters.
7. Ran compiler, runtime and broader regression checks. Scoped formatting to app
   source; reversed an initial formatter traversal of vendored Rune only where
   the file exactly matched rustfmt of its original revision. Vendor status is clean.
8. Inspected all five captures. GPU rendering needed an approved sandbox escalation
   because the sandbox could not obtain a Metal device.
9. Built the release app/ZIP with `python3.13 tools/package.py`; the packaging
   script verifies pinned dependencies and the ad hoc code signature.

Checks: 139 Rust all-target tests, Clippy with warnings denied, and all 37 Python
tests passed. Thirteen source-loader/highlighting tests and thirteen popup runtime
tests are included in the Rust total. Detailed Rust/Clippy output is stored alongside this document. Specialized
ignored analyzer/toolchain tests were not rerun because analyzers are unchanged.
The final renderer-footer extraction was additionally checked with the thirteen
popup runtime tests and Clippy. CodeScene reviewed modified source; the final safeguard checked 17 eligible files
and passed without regressions. Sonar is inactive: `.sonar/config.json` is absent.

Physical mouse/trackpad, native window-shell interaction and VoiceOver have not
been exercised. Pointer, wheel, magnification, clipboard and resize behaviors were
verified in the real Rune runtime, with native GPU screenshots for visual QA.
