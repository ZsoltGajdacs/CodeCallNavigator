# Validation record — 2026-11-08

## Delivery

`dist/CodeGraph.app` and `dist/CodeGraph.zip` were built with `tools/package.py`.
The executable is Mach-O arm64; deep strict ad hoc signature verification passes.
Pinned analyzer artifacts, relocated Graphviz/plugins/dylibs and third-party
notices are included. Production packaging rejects unexpected dependency revisions,
source changes and identity-patch checksum mismatches. SDKs/JDKs are not bundled.

With PATH restricted to `/usr/bin:/bin`, the packaged executable analyzed the
C# and Gradle fixtures together using bundled artifacts: 25 symbols and 26
relationships, successfully published to an isolated SQLite cache. It also rendered
the real mixed-project cache offscreen using bundled Graphviz. The packaged
interaction script rendered 331 GPU frames and exercised selection, directed paths,
pan/zoom, fit, keyboard focus, resizing and history. See `ui/packaged-interactions.png`
and `ui/packaged-real-overview.png`. Normal startup uses the user's workspace;
fixture data requires an explicit `--scene fixture` argument.

## Analyzer acceptance

PacificFate and StoryEngine source files were not modified. Their applications and
tests were not run. Real restores/compilation were permitted and performed.

| Build | Indexed symbols | Indexed relationships |
|---|---:|---:|
| PacificFate solution | 20,494 | 75,881 |
| StoryEngine backend | 14,504 | 22,326 |
| Combined app pipeline | 34,998 | 98,207 |

C# uses pinned CallAtlas with an identity serialization correction, preserving
compiler assembly ownership. Java uses existing java-callgraph2 with BCEL 6.13.0:
JDT LS failed the mandatory StoryEngine import gate and was rejected before the
UI milestone. Java 21 and 25 fixtures pass. Engine distinctions and missing bodies
remain explicit, with no adapter call resolution or DI inference. Exact signatures,
manual source checks and observed limitations are in `analyzers/ACCEPTANCE.md`,
`analyzers/native-pipeline.md` and the analyzer summary JSON files.

## Automated checks

- All-target Clippy with `--no-deps -- -D warnings`: passed.
- Rust: 34 tests passed, including the installed-JDK compiler regression normally
  marked ignored. Coverage includes graph paths/cycles/filters/budgets, real
  Graphviz edge identity, SQLite generations and failed refresh, descendant
  cancellation, freshness, pagination and native Rune interaction.
- Python: 20 tests passed, including rejected JDT responses/framing, actual Git
  bootstrap behavior and staged/unpatched dependency packaging rejection.
- CodeScene staged gate: passed, 45 eligible files checked with no issues.
  Introduced findings were refactored; final affected-file reviews score 10.
  See `code-health.json`. The original unborn-HEAD error was resolved with a
  metadata-only initial commit before checking the entire staged application.
- Sonar remains inactive because this new project has no `.sonar/config.json`.

## Scale measurement

Release benchmark: 250,000 method nodes and 2,000,000 relationships, 100 queries per
kind (including ten search prefixes matching all methods).

| Measurement | Result | Target |
|---|---:|---:|
| Cached graph open | 0.633 s | <5 s |
| Search p95 | 99.0 ms | <200 ms |
| Neighbor query p95 | 0.005 ms | <200 ms |
| Publication | 2.653 s | — |
| Peak process footprint | 488,440,648 bytes | — |

Hardware available here is an Apple M4 Pro with 48 GB RAM. This verifies the target
workload and latencies on that machine; the specified physical 32 GB machine has
not been tested. Raw measurements are in `performance/scale-final.json` and
`performance/scale-final-resources.txt` (the earlier run is also retained). Layout is asynchronous and excluded from cached
opening/query timing.

## Remaining physical validation

Headless Rune Metal rendering, light/dark screenshots and scripted interaction
pass. A live native-window probe rendered its first frame, then failed to obtain
a display link (-6661). CoreGraphics reports successful enumeration with **zero
active displays** in this execution session. See `ui/native-display-status.json`.
The partial native trace/log are failed-probe evidence, not a live interaction pass.

Physical trackpad navigation, keyboard focus and resizing in a visible macOS window
remain unverified. TASK-5 stays In Progress; neither this check nor the exact 32 GB
hardware benchmark is claimed complete. On a connected display, open the delivered
app, select a build, verify pan/pinch/scroll/fit, Tab/Cmd-F/Cmd-O and resizing; rerun
the release benchmark on a 32 GB Apple Silicon Mac.
