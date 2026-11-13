# Saved-edit graph freshness feasibility — TASK-59

Research date: 2026-11-13. This evidence implements the feasibility spike promoted
from DRAFT-3. It does not install a watcher, enable automatic refresh or change
production analysis. [Delivery contract and refactoring boundaries](../../docs/graph-freshness.md).

## Protocol

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

## Correctness and unavailable coverage

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

## Event observation and polling alternatives

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

## Recommendation and delivery tasks

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

## Reproduction

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

## Final measured results

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


## Verification

Full verification passed **all 32 stages** with explicit TypeScript, private
billing and saved-profile inputs. This includes compilation, Clippy, ordinary
and release/ignored real-toolchain acceptance, both Python suites, TypeScript
suites, storage and CPU/GPU checks, local packaging/signatures, installed CLI,
agent journeys, independently installed MCP and CodeScene. See
[stage results](verification/results.json), [summary](verification/summary.json)
and [quality log](verification/31-codescene.log).

Seven focused tests also passed independently, including real TypeScript
refresh/clean-reference/failure recovery and separately grouped process cleanup.
All six retained source files score 10.0 in CodeScene; the safeguard passes.
Sonar is inactive because this repository has no `.sonar/config.json`. No UI
behavior changed, so this spike makes no new desktop visual-acceptance claim.

The first full attempt stopped at preflight because the billing path was not
configured; after finding the saved project binding, the complete rerun passed.
Build artifacts were retained while measurement executables were active; final
reclamation runs the repository cleanup with native usage inspection.
