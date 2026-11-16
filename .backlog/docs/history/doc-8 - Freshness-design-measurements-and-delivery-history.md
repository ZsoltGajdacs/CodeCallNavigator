---
id: doc-8
title: Freshness design measurements and delivery history
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:13:00Z'
---
# Freshness design measurements and delivery history


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-freshness-readme-md"></a>

## evidence/freshness/README.md

Source: [evidence/freshness/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/README.md). Historical record; retained rationale, methods and limitations.

### Saved-edit graph freshness feasibility — TASK-59

Research date: 2026-11-13. This evidence implements the feasibility spike promoted
from DRAFT-3. It does not install a watcher, enable automatic refresh or change
production analysis. [Delivery contract and refactoring boundaries](<../../../docs/graph-freshness.md>).

#### Protocol

`tools/measure_freshness.py` creates isolated synthetic compiler projects, analyzes
a baseline, restores its exact saved inputs/artifact before every repetition,
performs the same saved edit, verifies staleness, runs full analysis and adopts
its published generation through a bound CLI status query. Build/dependency and
inference caches remain warm. A scenario has one first-repetition record labeled
`cold` and 20 warm repetitions; **cold does not mean clean dependencies or cold
filesystem caches**. Percentiles use nearest rank; 20 samples give a coarse p95.

The four routes are TypeScript, C#/.NET, Java/Maven and Java/Gradle. Synthetic
projects isolate authored behavior and are not representative large repositories.
The separate StoryEngine copy contains 685 original authored Java files plus a
two-method probe. The source checkout and live desktop store are never analyzed
or edited by the measurement harness. Full verification uses a snapshot of the
configured desktop profiling store.

Scenarios cover body/signature edits, a cross-file call target change, file
addition/deletion/rename, atomic replacement, ten-write bursts and configuration
file edits. TypeScript changes an effective compiler option. The other routes
change configuration-file comments: they prove byte-change detection, **not
effective build-configuration invalidation**. Dependency changes, generated-source
edits and multi-module invalidation remain production acceptance requirements.

Each record retains Complete stage metrics, CPU observations, 50 ms process-tree
RSS samples, status envelopes and query adoption latency. Nested stage times must
not be summed. CPU includes resource-sampling subprocess overhead; RSS may count
shared pages repeatedly and miss brief peaks. Short queries can finish before any
RSS sample. Detection is an immediate status request after the save, excluding
polling/event/debounce wait. Adoption is another CLI status query, not a desktop
frame. Publication has no independent timestamp in the current protocol.

Language matrix processes and verification workloads overlap on the same host; these are exploratory
contended-workload measurements, not isolated capacity benchmarks or release
latency commitments. Metadata records the exact binary digest, source revision,
analyzer lock, architecture and toolchain versions. The CLI was built in release
mode from the current checkout; the older packaged executable is not the matrix
baseline.
The metadata `source_commit` is the harness checkout revision, not an embedded
attestation for an arbitrary supplied executable. These runs used the preceding
successful local release build; the binary SHA-256 binds the actual measured file.

#### Correctness and unavailable coverage

The final harness compares complete canonical artifact bytes with a full analysis
of an independent clean temporary project, sharing only installed dependencies
and their download caches. It also asserts expected symbol membership/signature,
renamed source paths and changed call targets. Ordered flows, uncertainty,
provenance, diagnostics and input coverage remain part of strict equality.

The initial TypeScript/C# matrix used a second analysis in the same project:
those references establish repeatability. Separate clean-reference audits are
retained in `clean-audit/audit-conclusions.json`. All nine TypeScript scenarios
match the original complete reference hashes. All nine C# scenarios have equal
graph semantics but different provenance: the clean reference additionally
records `obj/Freshness.csproj.nuget.g.targets`. Common inputs match exactly.
Representative complete artifacts are retained as
`clean-audit/csharp-body-candidate.json` and `csharp-body-reference.json`.
This remains a strict equivalence failure; TASK-63 investigates it before C#
continuous-freshness support. No input fields were ignored to pass this check.

The C# adapter takes imported input paths from MSBuild's `MSBuildAllProjects`
property. A direct probe reproduces the discrepancy by rewriting the project
file with identical bytes after restore: the generated NuGet targets file drops
from that property. Timestamp-sensitive build metadata changes input coverage
even when authored content is unchanged; equal graph counts do not resolve it.
The initial Java discovery attempts failed because Java targets identify build
directories rather than configuration filenames. Those failed attempts are
retained rather than counted as samples. The first strict Gradle reference run
also exposed a difference requiring investigation; failed rows are not timings.
That difference was confined to presentation labels containing the checkout
basename. The final clean reference uses the same basename under a separate
temporary parent, retaining exact comparison of every field.

The large-corpus probe measures repeated body-edit detection/full refresh and
generation-bound query adoption. It explicitly has no independent clean
large-corpus semantic oracle; graph counts never substitute for that validation.
Generated sources may change between builds, and baseline artifact restoration
can therefore reintroduce stale generated-input observations as well as the
authored edit. It is a throughput/availability probe, not isolated incremental
invalidation proof.

Real-compiler invalid-code journeys verify a reported analysis failure retains
the prior artifact; wrong generation expectations are rejected. A real process
timeout regression verifies the measurement runner kills separately grouped
workers even when their parent exits on TERM. That test does not establish every
engine cancellation race. Desktop camera/history preservation, concurrent
reader/writer races, edits during publication, dropped-event reconciliation and
long-running watcher resource behavior remain unmeasured delivery journeys.

#### Event observation and polling alternatives

`events.json` records 20 native recursive `fs.watch` first deliveries for each of
seven edit shapes in tiny temporary directories. Filenames are correlated with
the mutated destination. The separate two-second idle sample records observer
process CPU and callback count. This measures event delivery only, not content
verification, large-tree coverage, event-loss recovery or production integration.

| Strategy | Detection wait before verification | Assessment |
| --- | --- | --- |
| Current 30-second polling | Uniform-phase model: mean 15 s, p95 28.5 s | Incompatible with one-second stale detection even when verification is fast |
| One-second polling | Model: mean 0.5 s, p95 0.95 s | Leaves almost no one-second budget for verification; repeatedly hashes unchanged inputs |
| 250 ms polling | Model: mean 125 ms, p95 237.5 ms | Useful fallback experiment; verification cost and resource use must still be measured at scale |
| Native events plus reconciliation | Native tiny-directory measurements in `events.json` | Recommended direction for prompt invalidation, with content verification and missed-event recovery |
| Debounced full refresh | Adds the chosen quiet interval and measured full-analysis cost | Safe initial automatic publication baseline; debounce/coalescing are not implemented by this spike |

Polling figures are mathematical wait models, **not measured polling runs**;
percentiles from independent stages cannot be added to claim an end-to-end p95.
No production idle polling or coordinator overhead is claimed. Evaluate a 250 ms
event quiet interval and five-second reconciliation interval in TASK-60/62;
adaptation or a different interval needs measured justification before release.

#### Recommendation and delivery tasks

Deliver fast, honest stale detection before attempting persistent compiler reuse.
The research objective is one-second p95 on each explicitly measured supported
workload. A watcher callback can invalidate `match` promptly, but cannot certify
the graph reflects current inputs. Keep `unverifiable` separate from stale.

Scale coverage is limited: the four controlled routes have two authored source
files and roughly 3–5 graph symbols. Only Java has a production-sized measured
copy (roughly 8,000 symbols); medium/large TypeScript and C#, larger Maven
reactors and graphs above 10,000 symbols are unmeasured here. No production
support claim follows from the microfixtures. Larger-project status queries
exceeded one second during contended runs, so the common detection objective is
not established at that scale.

For the next controlled experiments, use save-to-verified-observation p95 ≤1 s
and compare full-refresh-plus-query-adoption p95 against the per-workload baseline
below, allowing at most 10% regression under comparable isolated conditions.
This is a measurable experiment budget, not a shipped SLO; separate compiler
optimization requires an improvement rather than merely fitting that allowance.
Desktop frame responsiveness and long-lived idle-resource budgets require actual
consumer/monitor measurement before release.

TypeScript analysis records present/absent ancestor package-manifest probes up to
the filesystem root. Mapping installed `node_modules` is insufficient to map all
of those probes, so a just-refreshed synthetic graph can remain `unverifiable`
with `unmapped_input`. Do not paper over it by mapping `/`, dropping probes or
turning that result into `match`.
The Maven microfixture likewise remains `unverifiable`: consumed user/global
`settings.xml` and `toolchains.xml` observations are outside the project root.
Named explicit root mappings or a deliberate portable environment-observation
contract are needed; faster source events do not repair missing provenance.

- **TASK-60:** typed core/CLI observations and engine-owned detection/reconciliation;
  keep artifact/index reuse separate and preserve existing verification guarantees.
- **TASK-63**, a prerequisite of TASK-60: resolve/document the measured C#
  clean/warm input-coverage discrepancy without weakening the correctness oracle.
- **TASK-61**, depending on TASK-60: typed desktop/MCP consumption; split generation
  adoption from workspace reset so valid navigation and camera context survive.
- **TASK-62**, depending on TASK-60/61: opt-in automatic full refresh, one active
  analysis and coalesced pending work; isolate refresh cancellation from monitor
  shutdown, initially using existing one-shot analysis subprocesses.

TASK-55/56 remain separate reminder-hook experiments. Persistent TypeScript,
Roslyn or Java compiler reuse is not justified by a measured prototype here, so
no optimization implementation tasks or latency promises are invented. The
language-specific invalidation boundaries are recorded in the delivery contract.

#### Reproduction

Build a matching CLI and run outside the macOS sandbox for native watchers,
compiler caches and process-table resource sampling:

```sh
cargo build --locked --release -p codecallnavigator-engine --bin ccnavigator
python3.13 tools/measure_freshness.py \
  --executable target/release/ccnavigator --output /tmp/freshness-new \
  --typescript-modules /absolute/path/to/node_modules --runs 20
node tools/measure_freshness_events.js 20 > /tmp/freshness-events.json
```

The final all-route command intentionally exits nonzero on the known C# strict
clean-reference discrepancy and retains failed rows/artifact diagnostics. It
does not suppress that failure to reproduce the older C# repeatability timing
series. Use `--languages typescript,maven,gradle` for a sweep whose clean-reference
checks passed here; use `--languages csharp --runs 1` to reproduce the finding.
The original C# timing records remain available with their weaker oracle clearly
identified. Task completion means the feasibility findings are evidenced, not
that every assessed approach or analyzer passed its research oracle.

Each output directory must be new. Toolchain requirements are Node 24+ with
TypeScript 6.0.x, .NET SDK 11, JDK 25, Maven 3.9+/4 and the fixture Gradle wrapper.
No dependency caches are cleared. The selected TypeScript modules are linked
read-only in disposable projects; original source workspaces remain untouched.
Retained `samples.jsonl.gz` files are losslessly compressed raw JSON lines;
decompression was checked against the original SHA-256 before removing each
uncompressed copy. The harness itself emits plain `samples.jsonl`.

For the larger probe, first make a disposable copy of StoryEngine `backend/`
and `spec/` under the system temporary directory or `/private/tmp`. Exclude
`.git`, build/target outputs, `.gradle`, `node_modules`, `.idea`, `local.properties`,
`.env*` and private key files. Never point the probe at the original checkout:

```sh
python3.13 tools/measure_freshness_corpus.py \
  --executable target/release/ccnavigator \
  --project /private/tmp/isolated-storyengine-copy \
  --output /tmp/freshness-corpus-new --runs 20
```

The probe adds only its named Java source inside that disposable copy and refuses
an existing probe file. It leaves the copy for diagnosis; cleanup is explicit.

Measurement results are below; final verification is recorded separately.

#### Final measured results

All four fixture routes completed nine scenarios, each with one first-repetition
record and **20 warm repetitions**: **756 fixture samples**. The larger corpus
completed one first-repetition record plus **20 warm repetitions**. Native event
delivery has **140 records** (seven shapes × 20). All 777 selected CLI samples
have complete stage metrics and RSS observations without sampling errors.

The table uses **body edits only**, 20 warm samples per row. Status/adoption are
CLI process observations; refresh includes full analysis/publication. Conditions
vary over time and workloads overlap, so these rows are not controlled language
comparisons. The separate JSON summaries retain every scenario and percentile.

| Workload | Symbols / relationships | Status p95 | Refresh p95 | Adoption-query p95 | Warm peak tree RSS |
| --- | --- | --- | --- | --- | --- |
| typescript microfixture | 5 / 1 | 40.7 ms | 0.82 s | 41.0 ms | 419 MiB |
| csharp microfixture | 3 / 1 | 20.9 ms | 2.02 s | 20.9 ms | 264 MiB |
| maven microfixture | 3 / 1 | 23.3 ms | 4.99 s | 23.7 ms | 285 MiB |
| gradle microfixture | 3 / 1 | 21.2 ms | 13.12 s | 20.7 ms | 630 MiB |
| StoryEngine Java/Gradle copy | 8158 / 11147 | 2864.0 ms | 138.74 s | 3161.4 ms | 1296 MiB |

Native first-event p95 ranged from **39.9 to 41.8 ms** across the seven
shapes; the 2.00-second idle probe observed 0 callbacks.
These are tiny-directory event measurements and cannot be added to another
stage’s p95 to claim end-to-end performance.

TypeScript and Maven report `unverifiable` after refresh because of unmapped
external inputs; C# and the tiny Gradle fixture report `match`. The larger
Gradle copy also reports `unverifiable`, including external dependency JAR
observations outside its mapped root. Matching source bytes alone cannot
certify the complete recorded build environment.

Strict clean-reference comparison passes TypeScript, Maven and Gradle. C#
remains graph-equivalent with provenance metadata unequal (TASK-63). The larger
corpus has no independent semantic-equivalence claim.

The body-edit stage medians identify different cost centers. TypeScript spends
about 326 ms preparing and 343 ms extracting. C# spends about 1.30 s in its
compiler-backed engine, versus 208 ms restoring and 259 ms gathering metadata.
The Maven microfixture spends about 1.04 s preparing and 1.66 s compiling; the
Gradle microfixture spends about 4.62 s preparing and 5.03 s compiling. In the
larger copy, preparation is about 9.98 s, compilation 22.00 s, source attribution
20.75 s and bytecode analysis 7.45 s. These nested stage medians are observations,
not additive partitions. Boundary-decision cache reuse cannot eliminate these
compiler/build/source-attribution costs; no cross-edit compiler reuse was tested.

Initial TS/C# timings used summed adjacent operation durations for their
combined save-to-adoption field; final Gradle/corpus probes measure elapsed
time directly. The phase-level table above avoids treating those combined
fields as identical protocols.


#### Verification

Full verification passed **all 32 stages** with explicit TypeScript, private
billing and saved-profile inputs. This includes compilation, Clippy, ordinary
and release/ignored real-toolchain acceptance, both Python suites, TypeScript
suites, storage and CPU/GPU checks, local packaging/signatures, installed CLI,
agent journeys, independently installed MCP and CodeScene. See
[stage results](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/verification/results.json), [summary](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/verification/summary.json)
and [quality log](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/verification/31-codescene.log).

Seven focused tests also passed independently, including real TypeScript
refresh/clean-reference/failure recovery and separately grouped process cleanup.
All six retained source files score 10.0 in CodeScene; the safeguard passes.
Sonar is inactive because this repository has no `.sonar/config.json`. No UI
behavior changed, so this spike makes no new desktop visual-acceptance claim.

The first full attempt stopped at preflight because the billing path was not
configured; after finding the saved project binding, the complete rerun passed.
Build artifacts were retained while measurement executables were active; final
reclamation runs the repository cleanup with native usage inspection.


<a id="source-evidence-freshness-automatic-refresh-readme-md"></a>

## evidence/freshness/automatic-refresh/README.md

Source: [evidence/freshness/automatic-refresh/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/README.md). Historical record; retained rationale, methods and limitations.

### Opt-in automatic graph refresh

TASK-62 adds saved-file automatic refresh after TASK-63 input coverage, TASK-60
shared observation and TASK-61 consumer adoption. Automation is off by default.
The CLI owns verification, scheduling and compiler processes; desktop authorization
is scoped to the loaded canonical project for this session. MCP retains its
CLI-only, one-shot ownership boundary.

#### Measured publication behavior

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

The [derived metrics](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/metrics.json) retain distributions for detection,
scheduling and each reported compiler stage. Compiler stages are nested and
overlap; they must not be added together. The complete
[publication summary](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/publications/summary.json),
[metadata and reproduction command](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/publications/metadata.json), raw samples,
per-route stream traces and eight compressed reference artifacts are retained.

#### Resource observations

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
[idle summary](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/idle/summary.json) and [idle metadata](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/idle/metadata.json).

#### Reproduce

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

#### Correctness and structural boundaries

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

The [final acceptance matrix](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/acceptance/results.json) passed **14 real compiler,
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
manual analysis. See [desktop acceptance](<doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-task62-desktop-readme-md>) for the
native Rune/Metal journey and four inspected captures.

#### Scope and retained failures

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
[publications-initial](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/publications-initial/summary.json) and
[output-trigger diagnostics](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/output-trigger-diagnostics/summary.json). They exposed
the publication-generation race and Java's self-generated bytecode triggers.
Their counters, failures and partial samples are not mixed into final results.
Earlier acceptance checkpoints likewise retain their own executable hashes.

#### Verification

Core 106 tests, engine ordinary release tests, strict Clippy, focused protocol and
process tests, and independent reviews passed before the final campaign.
[CodeScene evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/code-health.json) records source hashes and findings.
The declaration-only gate also has 17 passing targeted regressions: the built-in
`cfg(unix)` predicate is recognized only on semicolon module declarations;
executable bodies and unknown attributes still require numeric review.
The `.gradle` extension cannot be scored by CodeScene; installation verification
passed, and real Gradle exporter/compiler fixtures cover the script.

All 92 full functional, performance, packaging, integration and CodeScene stages
passed. The final cleanup stage initially retained release artifacts because
native lsof returned an inconclusive result; the unchanged protected cleanup
passed on retry after confirming idle usage. The original full run exited 1;
[verification records](<doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-automatic-refresh-verification-readme-md>) preserve that result and the
separate successful cleanup. No product code changed after the measured freeze.


<a id="source-evidence-freshness-automatic-refresh-verification-readme-md"></a>

## evidence/freshness/automatic-refresh/verification/README.md

Source: [evidence/freshness/automatic-refresh/verification/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/README.md). Historical record; retained rationale, methods and limitations.

### TASK-62 full verification

The full run passed all 92 functional, performance, packaging, integration and
CodeScene stages. Its final cleanup stage returned an inconclusive native lsof
result and safely retained the release directory. The unchanged standard cleanup
command passed on retry after a native lsof inspection returned exit 1 with empty
stdout/stderr (idle). No protection was overridden. The original full command
therefore exited 1; these records document the completed checks and separate
successful operational cleanup, without rewriting the original result.

- [Original stage results](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/initial-results.json)
- [Original cleanup log](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/initial-93-cleanup.log)
- [Original cleanup report](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/initial-cleanup.json)
- [Successful cleanup retry](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/cleanup-retry.json)
- [CodeScene stage output](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/automatic-refresh/verification/codescene.log)

Full command, run outside the macOS sandbox:

```sh
CODECALLNAVIGATOR_RECURRING_BILLING=/path/to/private-maven-project \
CODECALLNAVIGATOR_TS_NODE_MODULES=/path/to/node_modules \
CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE=/path/to/CodeCallNavigator/target/freshness/task62-final-frozen/ccnavigator \
./verify-full.sh \
  --profile-source '/path/to/Application Support/CodeCallNavigator' \
  --timeout 3600 --output target/verification/task62-full
```

Cleanup retry (exit 0):

```sh
python3.13 tools/verification/cleanup.py \
  --root /path/to/CodeCallNavigator \
  --output target/verification/task62-full --mode full
```

The narrow declaration-only CodeScene parser correction was additionally checked
with all 17 tests in `tests/test_verification_quality.py`; both modified Python
files received numeric CodeScene 10 reviews. This verification-only change did
not modify the frozen executables used for publication/idle/acceptance evidence.


<a id="source-evidence-freshness-consumer-adoption-readme-md"></a>

## evidence/freshness/consumer-adoption/README.md

Source: [evidence/freshness/consumer-adoption/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/consumer-adoption/README.md). Historical record; retained rationale, methods and limitations.

### Desktop and MCP freshness adoption (TASK-61)

Desktop observes the loaded CLI project/root binding asynchronously. Matching,
stale, unverifiable and failed observations remain distinct. Editing the pending
folder does not retarget the monitor, Reload control or displayed workspace
identity. A newer saved generation offers explicit Reload; ordinary observation
does not analyze or automatically adopt it.

Same-project adoption preserves valid navigation, history and camera while
invalidating old generation/binding/epoch results. New projects reset the
workspace. Linked-root remapping preserves valid context but replaces hydrated
source addresses and rejects old bindings, even with identical portable bytes.
Deferred architecture assignments reconcile all layer/history/expanded targets
without late auto-fit or recentering.

The CLI snapshot owns displayed graph validity. One active/latest-pending worker
retains write-only profiling data; it never saves the session. Only an accepted
UI binding may install a cache marker and persist the current descriptor,
navigation and opaque settings. A local retention failure preserves the valid
displayed CLI graph. A cancelled publication can transiently leave a derived
active pointer, which is not a production graph-query source; marker validation
and the next accepted retention recover it.

#### Desktop verification

The actual TypeScript compiler/CLI/Rune/Metal journey covers saved edits, external
publication, explicit Reload, selection/history/camera preservation, obsolete
layout output, deleted selections, unverifiable provenance, terminal failure and
manual retry, and project changes. Header identity is checked before and after
editing the pending folder, Reload and the eventual project switch.

Seven native GPU captures and reproduction commands are retained in
[task61-desktop](<doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-task61-desktop-readme-md>), including a compact light-theme
failure/Retry state and an explicit fitted view. The parent visually inspected
the regenerated images. They are offscreen production Rune/Metal renders driven
by real CLI/compiler controls, not screenshots of a manually operated window.

Focused checks include real SQLite supersession/retention failure, actual CLI
linked-root remapping, capability retry after an in-place CLI upgrade, malformed
wire frames, all focus variants and delayed layer assignments. The ordinary root
release suite passed 240 tests; 75 projection integrations and release Clippy
also passed. Independent desktop validation covered 20 focused tests.

#### MCP verification

`graph_freshness` forwards the negotiated one-shot CLI observation unchanged,
including generation, live revision, validity, coverage, limitations and timing.
Independent DTO validation binds it to both the query envelope and explicit
legacy-summary conversion. MCP owns no watcher, graph cache or graph algorithm.

The new real stdio/compiler regression first failed with method-not-found and
then passed. The complete MCP release suite passed 36 tests, including existing
comparison, cancellation, disconnect and malformed-wire coverage. All nine
affected source files passed CodeScene at 10.0. The independent dependency/source
boundary check passed with 89 packages, plus four boundary tests.

[mcp-legacy-cli.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/consumer-adoption/mcp-legacy-cli.json) retains the actual pre-observation CLI
capabilities, binary hashes and native MCP responses: existing status remains
usable, only the optional freshness tool returns `unsupported_capability`, ping
and generation-bound status continue to work, and the graph bytes stay unchanged.
Both processes shut down normally. An independent tester reproduced these facts
against the same binary hashes.

Reproduce the complete MCP process suite with the current CLI built:

```sh
CODECALLNAVIGATOR_TEST_ENGINE="$PWD/target/release/ccnavigator" \
CODECALLNAVIGATOR_TS_NODE_MODULES=/path/to/pinned/node_modules \
cargo test --locked --release -p codecallnavigator-mcp \
  -- --include-ignored --test-threads=1
python3.13 tools/mcp_boundary.py --root .
```

Full workspace verification passed all **72 stages**, including the complete
release acceptance suite, benchmarks, packaging/signatures, installed CLI and
independent MCP journeys. The pre-commit CodeScene safeguard passed all 46
eligible source files with no regressions. See
[verification/results.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/consumer-adoption/verification/results.json) and the adjacent logs.


<a id="source-evidence-freshness-csharp-import-fix-readme-md"></a>

## evidence/freshness/csharp-import-fix/README.md

Source: [evidence/freshness/csharp-import-fix/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/csharp-import-fix/README.md). Historical record; retained rationale, methods and limitations.

### C# evaluated input coverage (TASK-63)

The real release-mode regression first failed on the existing analyzer:
`obj/Fixture.csproj.nuget.g.targets` disappeared after rewriting the project with
identical bytes. SDK 11.0.100/MSBuild 18.12.2 reports a timestamp-sensitive
`MSBuildAllProjects` value rather than a complete evaluated import inventory.
See the original causal probe in `../csharp-import-probe.json`.

Review also reproduced a consumed design-time-only import omitted by normal preprocessing: actual Roslyn exported DesignOnly but the manifest lacked Design.props. The old built engine also omitted both inner-framework property imports in a valid net10.0/net11.0 fixture. These were blocking input-coverage failures.

The analyzer now obtains actual evaluated imports from strict MSBuild preprocess
frames across normal, pinned Roslyn 5.9 design-time and inner-framework contexts, observes explicit generated/SDK/external files, and rechecks import
membership and content before metadata, after metadata and after CallAtlas.
The [input policy](<../architecture/doc-22 - Engine-architecture-and-analyzer-implementation.md#c-evaluated-build-inputs>)
documents the supported format and observed-input limitations.

#### Focused verification

- Before production changes: real cold/warm regression failed as expected.
- After changes: the same real regression passed, including independent clean
  full-artifact equality and identical-byte project rewrites.
- Real external nested property-only imports with `--` and `&` paths passed;
  false imports and unframed author comments were excluded. Imported property
  changes were stale, and an invalid imported file preserved the saved artifact.
- Real design-time imports and external linked source scopes are retained; new linked sources are stale. Both inner-framework property imports are retained, and a real solution covers SolutionDir-dependent imports.
- A real SDK round-trip preserves literal semicolon/comma/percent property values without overwriting DesignTimeBuild. The actual compiler consumes a relative design-time import from the special solution path. The separate native SolutionDir-expanded import limitation is retained in literal-property-probe.json; ordinary SolutionDir expansion remains an acceptance case.
- A real conditional IsTestProject/Compile AutoGen regression preserves configured
  normal-root flags while still collecting design-only imports and sources.
- A real external marker swaps A/B imports between normal/design contexts without
  changing file bytes or the union; the per-context inventory check rejects it.
- Real repeated preprocessing rejected a newly appearing external conditional import.
- An empty external referenced project retained its directory scope in the
  published manifest; adding its first source file was detected as stale.
- Parser tests cover root/parent framing, optional parentless end frames,
  escaped-file ambiguity and special paths.
- All 79 core unit tests passed; focused verification tests cover conservative
  legacy C# status, unchanged bytes, stale precedence and Java/TypeScript isolation.

`manifests.json` retains the fixed cold/warm observed-input manifests and version
report. The two canonical artifacts were exactly equal. External SDK imports are
captured in runtime fingerprints; those outside declared portable roots still
produce explicit `unmapped_input`, rather than a false verified match.

`cli-audit-v2/` retains the final public CLI audit under `msbuild-preprocess.v2`:
nine saved-edit scenarios, each with a first run and one warm repetition. All 18
successful refreshes matched independent clean analysis exactly; the harness also
checked invalid-code recovery and generation mismatch rejection. These are
correctness checks, not a new latency distribution. `cli-audit/` retains the
earlier prototype audit for comparison. Raw samples in both directories are
losslessly compressed as `samples.jsonl.gz`; metadata records reproducible
commands and toolchains. The earlier TASK-59 timing series remains unchanged.

Reproduce with installed .NET 11 and pinned analyzer resources:

```sh
CODECALLNAVIGATOR_CSHARP_INPUT_EVIDENCE=/tmp/task63-manifests.json \
  cargo test --locked --release -p codecallnavigator-engine \
  --test csharp_inputs -- --ignored --nocapture
cargo test --locked --release -p codecallnavigator-engine \
  --lib analyzers::csharp::inputs -- --include-ignored
cargo test --locked --release -p codecallnavigator-core --lib
```

Full workspace verification passed all 36 stages with `--timeout 3600`, including
the complete release acceptance suite, profiling snapshot/benchmarks, packaging
and signatures, installed CLI and independent MCP journeys, and CodeScene.
The default 1800-second release-stage limit was insufficient for the two private
Maven builds plus the remaining compiler fixtures; that timeout report is also
retained. No assertion failed in that attempt. See [verification/results.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/csharp-import-fix/verification/results.json)
for the successful retry and the adjacent quality/install logs. The pre-commit
safeguard passed all ten modified source files with no regressions.


<a id="source-evidence-freshness-observations-readme-md"></a>

## evidence/freshness/observations/README.md

Source: [evidence/freshness/observations/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/observations/README.md). Historical record; retained rationale, methods and limitations.

### CLI-owned saved-input observations (TASK-60)

The production `freshness` query and `observe` stream use the same core manifest
verification pass. The query executor owns saved artifact/index reuse; observation
does not construct the graph index. The stream owns sequential full reconciliation
with a nominal 250 ms delay between passes and a five-second heartbeat. Its bounded
output coalesces current state rather than providing a durable event journal.

This is explicitly limited `verified_polling`, without native event acceleration.
Every emitted status comes from verification. Generation identifies immutable
saved graph bytes; input revision identifies current observed contents and scope
membership, including later edits to newly added files. Volatile observations are
not written into canonical artifacts. See the [wire contract](<../../../docs/cli-queries.md>).

#### Measured fixture results

Each route ran all nine existing scenarios: body, signature, cross-file, addition,
deletion, rename, atomic save, burst and configuration. Each scenario has one first
sample followed by 20 warm repetitions. All **756 records** passed final-revision,
generation, sequence, receipt-time and shutdown checks. All 36 scenarios met the
one-second p95 research objective on these small fixtures.

| Compiler route | Warm samples | Worst scenario p95 | Body fixture nodes / observed inputs |
| --- | ---: | ---: | ---: |
| TypeScript | 180 | 262.6 ms | 5 / 91 |
| C# | 180 | 264.0 ms | 3 / 8 |
| Java/Maven | 180 | 264.9 ms | 3 / 21 |
| Java/Gradle | 180 | 264.7 ms | 3 / 22 |

Fixtures contain two authored source files, or three for deletion/rename baselines.
This measures save-start to receipt of a complete verified stream line, including
the saved-write duration. The final edited revision is established independently
through a one-shot query before starting the stream. Each repetition consumes a
later edited event and then a later restored-baseline event. First samples are
reported separately; they are not cold dependency or compiler starts.

All recorded scopes were completely observed in this run. TypeScript, C# and Maven
baselines remain **unverifiable** because inputs outside declared portable roots
remain explicitly `unmapped_input`; Gradle baselines matched. Complete scope
coverage does not certify the build environment. No larger-project latency target
is claimed: all modes remain explicitly limited, and scan duration adds to the
nominal polling delay. Unrecorded inputs, unsaved buffers and nested directory
links outside explicit scopes retain the documented limits.

`fixture-series/samples.jsonl.gz` contains the losslessly compressed raw records.
`summary.json` retains per-scenario distributions, baseline analysis metrics,
artifact/source/input sizes, reference observations and process-tree memory/CPU
reports. `metadata.json` records the executable/analyzer hashes, toolchains,
hardware and exact command. CPU totals include startup and resource-sampler child
processes; they are not isolated steady-state observer CPU measurements.

Reproduce with installed analyzer resources and a new output directory:

```sh
cargo build --locked --release -p codecallnavigator-engine --bin ccnavigator
python3.13 tools/measure_freshness_observer.py \
  --executable target/release/ccnavigator \
  --typescript-modules "$CODECALLNAVIGATOR_TS_NODE_MODULES" \
  --runs 20 --output /tmp/ccn-observer-evidence-new
```

#### Correctness and review

Core regressions and independent probes cover stale-to-stale repairs, nested new
files, missing inputs, linked roots, per-scope exclusions, explicit generated
inputs, Maven output markers, directory and file symlinks, broken-link retargets,
device/FIFO rejection and active hashing cancellation. Unix input hashing uses
nonblocking opens and validates the opened descriptor before reading regular
files, with cancellation checks per chunk.

Real CLI/compiler journeys cover generation replacement and bound-generation
rejection, malformed-artifact failure with explicit restore/restart, capability
negotiation, legacy one-shot/session behavior, heartbeat, EOF and broken stdout.
An undrained full stdout pipe does not prevent EOF shutdown. Independent review
also removed an unbounded acknowledgement queue before measurement collection.

Core release tests: 89 passed. Independent CLI checks: six freshness, seven
legacy query (including real TypeScript) and two session tests passed. The eleven
measurement/compiler/process regressions passed. Modified eligible source files
passed CodeScene review and the pre-commit safeguard (24 files, no regressions).
Full workspace verification passed all **50 stages**, including the complete
release suite, benchmarks, packaging/signatures, installed CLI and independent
MCP journeys. See [verification/results.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/observations/verification/results.json) and the
adjacent quality/install logs. The initial formatting-only failure is retained;
the successful retry used scoped Rust formatting and `--timeout 3600`.


<a id="source-evidence-freshness-task61-desktop-readme-md"></a>

## evidence/freshness/task61-desktop/README.md

Source: [evidence/freshness/task61-desktop/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task61-desktop/README.md). Historical record; retained rationale, methods and limitations.

### Desktop freshness and explicit adoption

These images come from actual TypeScript compilation, the public CLI observer and
snapshot commands, production Rune controls, and native Metal offscreen rendering.
Temporary fixture checkouts and the SQLite profiling store are isolated. The
folder editor intentionally points at another temporary project while the initial
graph, workspace header and monitor remain bound to the loaded project. The
journey asserts the header before/after folder editing, after adoption and after
a real project switch.

| Image | Observed state |
| --- | --- |
| `stale.png` | Saved body edit observed as stale; existing selected method remains displayed. |
| `reload-offered.png` | External analysis publishes another generation; adoption requires the Reload control. |
| `adopted.png` | Selected symbol/history and the deliberately nondefault camera survive adoption. |
| `unverifiable.png` | The real compiler graph is republished as a supported legacy artifact without input provenance; observation reports `missing_analysis`. |
| `deleted-selection.png` | Another compiler refresh removes the selected method; obsolete selection/focus disappear and camera remains stable. |
| `fitted-view.png` | An explicit user Fit shows the remaining graph at a readable scale. |
| `failed-light-compact.png` | Removing the saved artifact terminates observation with contextual failure; the loaded graph remains visible and Retry stays available at 1050×680 in the light theme. Restoring the file and clicking Retry recovers. |

The first five camera-preservation captures intentionally retain the test's pan
and zoom even when that positions part of the graph under the fixed controls.
`fitted-view.png` uses an explicit user action after those assertions. Freshly
compiled TypeScript observations can also be unverifiable because the fixture
records unmapped dependency inputs; they are not converted to matching status.

Reproduce from the workspace root with the engine built:

```sh
CODECALLNAVIGATOR_TS_NODE_MODULES=/path/to/pinned/node_modules \
CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES="$PWD/evidence/freshness/task61-desktop" \
cargo test --locked --release -p codecallnavigator --bin codecallnavigator \
  actual_cli_freshness_reload -- --ignored --nocapture
```

Metal access outside the macOS sandbox is required when captures are requested.
The final capture journey passed in 5.77 seconds. The capture helper invalidates
its font atlas for each new offscreen renderer so subsequent images upload the
same fonts correctly; no image pixels are edited afterward.

Focused release checks additionally cover actual CLI cancellation and capability
renegotiation after an in-place upgrade, malformed/contradictory wire frames,
all presentation focus variants, linked-root remapping with identical portable
bytes, delayed layer assignments, and real SQLite retention. Retention tests
hold the existing writer lock to supersede an active and a pending request,
then verify only the latest accepted descriptor/cache marker is persisted while
newer camera and opaque settings survive. A deliberate cache-directory failure
preserves the valid displayed CLI graph and reports no successful marker.

The root ordinary release suite passed 240 tests with nine explicit ignored
acceptance/benchmark tests. Independent source review and focused desktop/MCP
validation passed. Full workspace verification passed all 72 stages; the parent
record retains [results and quality logs](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/consumer-adoption/verification/results.json).


<a id="source-evidence-freshness-task62-desktop-readme-md"></a>

## evidence/freshness/task62-desktop/README.md

Source: [evidence/freshness/task62-desktop/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/README.md). Historical record; retained rationale, methods and limitations.

### Automatic-refresh desktop acceptance

The focused release test passed on 2026-11-13 in 15.19 seconds using real
TypeScript compilation, the installed CLI process contract, Rune controls and
native Metal captures. Exact binary and image hashes are in [run.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/run.json).
This is a desktop journey, not a language-wide latency benchmark or a replacement
for full verification.

The journey proves passive observation does not analyze/adopt; explicit session
opt-in stays bound to the loaded canonical project despite a different pending
folder; publication uses generation-bound adoption and preserves the camera;
invalid code preserves the previous graph, a saved repair recovers; manual
analysis waits for automatic cleanup; and opting out returns to passive behavior.

#### Reproduce

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

#### Visual review

The captures were inspected. Opt-in and cancellation controls remain accessible;
freshness validity and refresh activity are separate. Failed compilation has a
concise label with full diagnostics available through its tooltip, including in
the compact light theme. Automatic adoption intentionally retains the test's
panned/zoomed camera, so these captures do not demonstrate an explicit Fit action.

- [Enabled, dark wide](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/automatic-enabled.png)
- [Invalid code, dark wide](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/automatic-invalid-code.png)
- [Invalid code, light compact](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/automatic-invalid-light-compact.png)
- [Disabled, dark wide](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/task62-desktop/automatic-disabled.png)
