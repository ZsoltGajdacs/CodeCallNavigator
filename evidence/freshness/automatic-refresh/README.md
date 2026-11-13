# Opt-in automatic graph refresh

TASK-62 adds saved-file automatic refresh after TASK-63 input coverage, TASK-60
shared observation and TASK-61 consumer adoption. Automation is off by default.
The CLI owns verification, scheduling and compiler processes; desktop authorization
is scoped to the loaded canonical project for this session. MCP retains its
CLI-only, one-shot ownership boundary.

## Measured publication behavior

The final frozen release CLI completed **84 automatic publications**: one first
publication and 20 warm repetitions for each route. Every result byte-matched
one of two independently analyzed clean-project references. Saves alternate body
variants; no manual analysis occurs between automatic publications. Retained
attempt counters prove one actual analysis start per measured edit.

These are two-authored-file fixtures, not large-project latency guarantees.
TypeScript also observes its declared dependency root. Caches and toolchains are
warm; “first” is the first timed publication, not a cold dependency cache.

| Route | First | Warm median | Warm p95 | Warm maximum | Initial local p95 budget |
| --- | ---: | ---: | ---: | ---: | ---: |
| TypeScript | 1.547 s | 1.546 s | 1.557 s | 1.559 s | 2.0 s |
| C# | 7.193 s | 7.205 s | 7.455 s | 7.457 s | 9.5 s |
| Java/Maven | 3.100 s | 3.344 s | 3.371 s | 3.586 s | 6.0 s |
| Java/Gradle | 6.706 s | 6.951 s | 7.195 s | 7.216 s | 14.0 s |

Timing ends when the client receives a verified observation of the expected new
generation. Saved bytes then remain stable until retained Published activity
delivers compiler metrics. This extra collection interval is outside the timed
publication latency. Exact atomic-rename and desktop-frame timestamps remain
unavailable.

Budgets are fixture-specific starting points, rounded up to 500 ms from
`max(current warm p95 × 1.25, TASK-59 manual body-refresh p95 + 550 ms)`.
The 550 ms represents nominal reconciliation plus debounce. TASK-59 manual p95
values were 0.817/2.015/4.986/13.120 seconds respectively. Those older campaigns had
different contention, and C# predates TASK-63's stronger import coverage; these
numbers do not establish a speedup or regression attributable to automation.

Warm stale-detection p95 was **263.3/261.1/261.7/262.9 ms** respectively, meeting
the one-second research objective on these fixtures. Configured debounce is
300 ms; additional scheduling/queue wait p95 was approximately 223–225 ms.
Supervised analysis p95, including owned-process cleanup, was
0.783/6.602/2.648/6.309 seconds. C# metadata observation accounts for 4.136 seconds
p95, while its compiler engine accounts for 1.188 seconds. Compiler reuse remains
within one analysis; no cross-edit incremental compiler state was introduced.

The [derived metrics](metrics.json) retain distributions for detection,
scheduling and each reported compiler stage. Compiler stages are nested and
overlap; they must not be added together. The complete
[publication summary](publications/summary.json),
[metadata and reproduction command](publications/metadata.json), raw samples,
per-route stream traces and eight compressed reference artifacts are retained.

## Resource observations

| Route | Stream CPU, including sampler | Sampled peak process-tree RSS | Inputs | Symbols / relationships |
| --- | ---: | ---: | ---: | ---: |
| TypeScript | 40.91 s | 395.5 MiB | 91 | 5 / 1 |
| C# | 187.66 s | 277.5 MiB | 8 | 3 / 1 |
| Java/Maven | 141.81 s | 305.3 MiB | 21 | 3 / 1 |
| Java/Gradle | 387.16 s | 646.9 MiB | 22 | 3 / 1 |

Stream totals cover initialization and all 21 publications. CPU uses
`RUSAGE_CHILDREN`, including measurement sampler processes. RSS sampling waits
50 ms between subprocess samples; shared pages may be counted repeatedly and
brief peaks can be missed. All four streams ended with exit 0 and no sampling or
cleanup errors.

Separate idle measurement uses one ten-second window per route and mode after a
real baseline analysis and monitor readiness, with no saved edits or compiler
activity. All eight monitors preserved the artifact, stayed healthy, and shut
down cleanly.

| Route | Passive idle CPU | Automatic idle CPU | Passive / automatic peak RSS |
| --- | ---: | ---: | ---: |
| TypeScript | 3.7% | 3.9% | 5.1 / 5.3 MiB |
| C# | 0.3% | 0.4% | 4.8 / 5.0 MiB |
| Java/Maven | 0.7% | 1.1% | 5.1 / 5.4 MiB |
| Java/Gradle | 0.8% | 1.3% | 5.0 / 6.4 MiB |

Idle CPU is CLI self CPU as a percentage of **one core**, excluding helper CPU.
`ps` has centisecond resolution; endpoint overhead is reported separately. These
single windows are exploratory averages, not p95 claims. See the
[idle summary](idle/summary.json) and [idle metadata](idle/metadata.json).

## Reproduce

Install the documented compiler/toolchain prerequisites, rebuild matching analyzer
resources, and use fresh output directories. Run outside the macOS sandbox:

```sh
python3.13 tools/bootstrap.py
cargo build --locked --release -p codecallnavigator-engine --bins
python3.13 tools/automatic_refresh_acceptance.py \
  --executable target/release/ccnavigator \
  --typescript-modules /absolute/path/to/node_modules \
  --output target/freshness/reproduce-acceptance
python3.13 tools/measure_automatic_refresh.py \
  --executable target/release/ccnavigator --runs 20 \
  --typescript-modules /absolute/path/to/node_modules \
  --output target/freshness/reproduce-publications
python3.13 tools/measure_freshness_idle.py \
  --executable target/release/ccnavigator --seconds 10 \
  --typescript-modules /absolute/path/to/node_modules \
  --output target/freshness/reproduce-idle
```

Do not rebuild executables or resources during a campaign. The retained metadata
identifies the frozen executable, analyzer resources, hardware, toolchains and
exact measured commands. It records the preceding Git HEAD plus a working-tree
qualification; the completed task commit retains the measured implementation.

## Correctness and structural boundaries

- Exact selected target identities are recorded before analysis consumes them.
  Same-directory Maven and Gradle identities are preserved; old path lists are
  never used to guess build intent.
- The query executor retains artifact/index ownership. Shared core fingerprinting
  supplies full observations and the validated Java scheduling view; the engine
  owns debounce, retries and one-shot compiler supervision.
- One coordinator lease and the existing publication lock protect competing
  clients. Each automatic child checks its planned generation under that lock.
  One active analysis and one latest pending binding are allowed.
- Every refresh uses a new process. Supersession, user cancellation and monitor
  shutdown are distinct; the process-global cancellation flag is never reset.
  Request writes, output readers, descendant identities and cleanup are bounded.
  Cleanup uncertainty remains visible, including after desktop worker retirement.
- Desktop opt-in follows the loaded project, independent of the pending folder.
  Adoption uses the generation-bound reload path and preserves valid camera and
  navigation state. Explicit manual analysis freezes its request and waits for
  automatic cleanup.
- Java compiler outputs stay in full public freshness verification. Only validated
  evaluated class directories are excluded from scheduling. Original per-input
  exclusions and filtered directory baselines preserve sibling sources and
  dependency bytecode. Raw adapter bindings, `binding_policy: direct.v1`, and
  per-pass symlink checks prevent hidden input aliases.
- Refresh activity has a nullable full observation revision; a missing public
  revision is not replaced by an invented digest. A separately validated filtered
  trigger can remain usable while owned outputs are temporarily unreadable.

The [final acceptance matrix](acceptance/results.json) passed **14 real compiler,
filesystem and process cases**: burst coalescing, invalid-code suppression and
repair, manual and coordinator contention, generated outputs, explicit cancel,
EOF/SIGTERM/broken-pipe shutdown, undrained-output shutdown, edits during analysis,
legacy/future selection and unsupported observation versions. The undrained case
verified 65,536 queued bytes and clean descendant shutdown before helper fallback.
It does not infer an event journal from coalesced notifications.

Independent source review additionally exercised full versus filtered filesystem
coverage, transient owned FIFOs, custom outputs, converging exclusion policies,
protected dependencies, raw output aliases and nullable activity. Real Maven and
Gradle exporter fixtures verify raw versus canonical output paths and successful
manual analysis. See [desktop acceptance](../task62-desktop/README.md) for the
native Rune/Metal journey and four inspected captures.

## Scope and retained failures

Only saved filesystem changes are supported. Monitoring remains explicitly
limited verified polling, with no native event backend. SDK/unmapped input
limitations can keep public freshness unverifiable even after successful
publication; observed-scope completeness does not certify the build environment.

Java automation requires exact selection and complete certified output ownership.
Old or future policies, source/configuration/dependency overlap, symlink-bearing
output bindings and mixed Java with another language use explicit manual refresh.
Generated sources and non-owned dependency bytecode remain triggering inputs.
Compiler-generated semantic source changes can require another full analysis.
Larger-project automatic publication distributions, unsaved buffers, steady-state
desktop CPU, and a universal publication SLA are not established here.

Earlier failed measurements are retained in
[publications-initial](publications-initial/summary.json) and
[output-trigger diagnostics](output-trigger-diagnostics/summary.json). They exposed
the publication-generation race and Java's self-generated bytecode triggers.
Their counters, failures and partial samples are not mixed into final results.
Earlier acceptance checkpoints likewise retain their own executable hashes.

## Verification

Core 106 tests, engine ordinary release tests, strict Clippy, focused protocol and
process tests, and independent reviews passed before the final campaign.
[CodeScene evidence](code-health.json) records source hashes and findings.
The declaration-only gate also has 17 passing targeted regressions: the built-in
`cfg(unix)` predicate is recognized only on semicolon module declarations;
executable bodies and unknown attributes still require numeric review.
The `.gradle` extension cannot be scored by CodeScene; installation verification
passed, and real Gradle exporter/compiler fixtures cover the script.

All 92 full functional, performance, packaging, integration and CodeScene stages
passed. The final cleanup stage initially retained release artifacts because
native lsof returned an inconclusive result; the unchanged protected cleanup
passed on retry after confirming idle usage. The original full run exited 1;
[verification records](verification/README.md) preserve that result and the
separate successful cleanup. No product code changed after the measured freeze.
