# C# desktop workflow acceptance

TASK-22.4 uses the production CLI-backed workspace, with no separate desktop
language gate or UI-side inference about missing flow inventories.

## Verified focused behavior

- Tests-first presentation checkpoint: 9 passed and two expected failures
  (unlocated selection reset source scroll; a known unavailable body kept an
  enabled action). After the bounded fixes, all 11 pass.
- Three release-mode real-CLI desktop workflows pass. The genuine .NET fixture
  preserves BOM, CRLF and Unicode; checks cover source spans, branches, implicit
  cleanup, exact overload targets, possible interface dispatch, target/Back and
  independent camera/scroll history, cached source after edit/deletion, partial
  async and unavailable abstract flows, older inventories and minimum-window
  light/dark layout. These are headless Rune tests, not displayed native windows.
- The real CLI epoch test proves immediate cancellation. Three controlled
  publisher tests additionally prove that late Ready/Error results actually land
  and are rejected for obsolete serial, epoch or symbol without changing the
  current session/cache; a matching-result positive control publishes.
- Eleven native-probe support tests pass, including: strict launch guards, split-frame
  production pointer dispatch, and screenshot acknowledgment validation.
- Independent review approves the implementation and evidence. All 20 touched
  source files score 10 in [code-health.json](code-health.json). The default
  verifier CLI is older than MCP and initially flagged two probe functions;
  cohesive reply/assertion refactors preserve the checks and now pass both
  versions. The exact corrected gate is [retained](quality-corrected-cli.json).

## Measurements

[refresh-overhead.json](refresh-overhead.json) records three alternating warm
refresh samples after one warmup per engine, using isolated copies of the named
fixture and a rebuilt pre-feature `6071db0` CLI with its pinned analyzer patch.
Baseline median: 6.767 seconds and 17,288 artifact bytes. C# flow candidate
`bff3d27`: 7.772 seconds and 24,169 bytes. Existing canonical graph nodes,
relationships and entry points match. The delta includes the whole C# feature
set; it does not isolate exporter CPU cost or establish statistical significance.

## Native and relocated evidence

[The final native run](native-minimum-managed/result.json) passes in both themes
at an observed content viewport of exactly **1050 by 680**. Owned-window images
were inspected: native title-bar buttons are clear, the selected `Leaf(7)` step
highlights source line 116, overload/certainty details remain visible, source
scroll does not move the chart camera, and target/Back/graph restoration pass.
The C# fixture has no unlocated step, so that check is explicitly null here;
the separate unlocated UI regression proves unchanged scroll and target access.

The opt-in probe uses normal saved-descriptor boot and production CLI queries.
Only actual native Drawn frames advance the assertions. A background worker
handles startup/capture acknowledgments and report files. The harness relocates
the complete app, clears analyzer overrides, removes .NET from the GUI PATH,
checks bundled engine/Graphviz paths, and captures only its verified window.
Input is synthetic through production Runtime handlers in a real native window;
OS mouse delivery is not measured.

The successful run records queued pointer-press to first ready Drawn frame:
root charts 136.4/119.6 ms and followed targets 54.8/53.0 ms (dark/light).
These are two observed samples, including CLI/layout work, not latency guarantees.
First-ready sampling occurs during gesture settling instead of imposing its
100 ms delay on the measurement.

AeroSpace initially resized the window after creation. The external harness now
queries only its owned PID and verifies the exact native window ID before using
AeroSpace's per-window floating command. Both final audits record `h_tiles` to
`floating` for their own window and cleanup after the owned app exited. No global
rules/preferences or existing windows are changed. The probe issues one resize
only after setup acknowledgment, waits for 250 ms of matching Drawn sizes, and
rejects subsequent size or window-ID changes. Native flow headers consume Rune's
existing title-bar inset. Nine Python ownership/cleanup tests and eleven probe
support tests pass.

Earlier failures remain available: `native-first` could not render while every
display slept; `native-awake` woke displays but the session was locked.
[native-prerequisites.json](native-prerequisites.json) records timestamped SDK
results. After the user unlocked the desktop, `native-unlocked` passed functional
checks at a larger viewport; it is not minimum-size evidence and its older timing
included gesture settling. `native-minimum` correctly rejected AeroSpace's later
resize. The final managed-window run supersedes those attempts without rewriting
history.

[Relocated offscreen evidence](relocated-offscreen-final/verification.json) also
passes the real C# CLI-backed UI test using the relocated engine and Graphviz,
with independently inspected light/dark minimum renders. It is labeled headless
GPU rendering, separate from the native proof. The first extra offscreen attempt
is recorded as not run because verifier cleanup had already removed its binary.

Repeat with an unlocked desktop and a fresh output directory:

```sh
/usr/bin/caffeinate -u -d python3.13 tools/csharp_desktop_acceptance.py --app dist/CodeCallNavigator.app --output /private/tmp/csharp-desktop-new-run
```

The display assertion ends with the command. Inspect both owned-window images.
Full verification `full-0tid44xs` passed all 46 stages outside the macOS sandbox
with isolated profiling. [verification.json](verification.json) retains the final
results and earlier failed/corrected checkpoints.
