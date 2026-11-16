---
id: doc-6
title: Engine artifact query and MCP verification history
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:06:00Z'
---
# Engine artifact query and MCP verification history


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-engine-readme-md"></a>

## evidence/engine/README.md

Source: [evidence/engine/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/README.md). Historical record; retained rationale, methods and limitations.

### Standalone engine reference

The pre-refactor reference is local `master` merge
`fb54f5f789d94c6cbc5fe86bf20010edea815b33` (parents preserve the previous master
and `codex/function-flowcharts` at `8c69c6e`). Implementation starts on
`codex/standalone-graph-engine`. No remote branches were changed.

The complete release executable and matching adapters were frozen at
`/private/tmp/codegraph-reference-fb54f5f/`, using the app resource layout.
Rebuild from the reference commit with `tools/bootstrap.py` and
`cargo build --release --locked --bin codegraph`; copy the executable and the
CallAtlas, Java-bytecode, TypeScript, Gradle metadata and Maven metadata resources
together, as `tools/package.py` does. The temporary copy is a convenience; the
commit and checked-in dependency lock/patches are the durable reference.

Baseline verification:

- Dependency revisions and complete source patches match `analyzers.lock.json`.
- `cargo test --all-targets` passed.
- Python suite: 50 passed. TypeScript real-compiler suite: 46 passed.
- Including ignored library tests: 50 passed; the billing compiled-output test
  requires external `CODEGRAPH_COMPILED_METADATA` and could not run.
- Strict Clippy has four pre-existing warnings: a range loop in
  `src/edges/viewport.rs`, and constant assertions in the UI performance tests.
- CodeScene pre-commit safeguard passed with no modified files.
- No `.sonar/config.json` is configured.

The corpus manifest specifies existing authored fixtures rather than generated
build products. Full comparison includes flow and architecture payloads as well
as nodes, edges, entries, diagnostics and freshness facts. Build-generated content
volatility produces an explicit differing-field result; it is never silently
removed from graph semantics. The TypeScript fixture records the installed exact
direct dependency versions; the runner uses existing dependencies and never
installs packages. Each analysis additionally records evaluated engine versions.

#### Foundation gate — 2026-11-10

The complete extraction matched the frozen native payload on Maven, Gradle, C#,
TypeScript and mixed workloads. `extraction-comparison.jsonl` records the first
check; `foundation-performance.jsonl` records three further comparisons per
workload against the frozen release and the packaged standalone release. Every
comparison matched. Only unordered graph collections and SQLite representation
details are normalized; statement/control-flow order stays strict.

The timing medians did not regress on this corpus. These measurements compare
headless native output against baseline SQLite publication; they do not measure
desktop rendering or claim a production speedup. Memory was not sampled in this
reference check.

`canonical-comparison.jsonl` records repeated and relocated exports (including
reversed selected-target order). The initial Gradle relocation test exposed
checkout-derived project display names; canonical export now maps that prefix to
the logical root and hydration restores the local display name. All five cases
then produced identical bytes.

Full workspace tests and strict workspace Clippy passed. Real-toolchain
acceptance passed with private billing-data tests and explicit GPU/CPU profiling
benchmarks excluded. Those prerequisites are unchanged from baseline. The four
pre-existing Clippy diagnostics were resolved without changing graph behavior.
Python tests passed (53 at foundation integration); TypeScript adapter and flow
tests passed, including semantic identities under comments and movement.

Both distributions were built, and the app was code-signed and verified.
`packaged-smoke.json` records identical output from packaged standalone analysis
and the packaged desktop compatibility CLI. Actual-executable tests cover invalid
protocol/version, EOF, idle SIGTERM, active cancellation, descendant reaping,
failed-publication preservation, linked sources and stalled terminal responses.
Detached-child forced cleanup was verified outside the macOS sandbox; that
specific regression is ignored by default because sandbox process-table access
is denied. Runtime reports when forced cleanup cannot be verified.

Each acceptance fixture now has one checked-in `.codegraph/graph.json`; the mixed
artifact belongs to `fixtures/`. Update them with
`python3.13 tools/update_engine_corpus.py --executable /path/to/codegraph-engine`,
then review changes with the engine's Git comparator before committing. The
updater never installs dependencies, stages files or creates commits.

Foundation review found and fixed two client boundary regressions: discovery
before desktop boot finished, and linked project-owned sources outside the
selected folder. The machine protocol now derives stable logical aliases from
compiler-owned paths for the desktop; explicit human CLI roots remain strict.
The legacy timing destination rejects graph publication paths and symlink aliases.

#### Resolution checkpoints and final verification

The foundation gate preceded every resolution stage. Subsequent checkpoints:

| Checkpoint | Verification |
| --- | --- |
| `91e666f` — explainable evidence | `tests/relationship_evidence.rs` and production inspector tests |
| `00db2bc` — observer channels | `tools/typescript/observer.test.js`; registration alone adds no invocation |
| `a22e651` — callable propagation | `tools/typescript/callable.test.js`; native bindings and graph/flow agreement |
| `008575b`, `b7207e2` — Spring and MediatR | `tests/java_spring.rs`, `tests/mediatr_dispatch.rs` and qualified-type controls |
| `3f8b652` — navigation and HTTP | [Boundary evidence](<doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-stages-boundaries-md>) |
| `b839ba3` — incremental decisions | [Incremental evidence](<doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-stages-incremental-md>) |

The final packaged engine reproduces all seven stored corpus graphs byte for
byte against the boundary checkpoint. Cold/warm, reordered and relocated runs
agree; failure and cancellation preserve successful graph/cache state. Controlled
final runtime medians stay within 1.5% of the previous checkpoint. Both measurement
rounds are retained because JVM RSS/timing varied substantially; no general
speedup or memory improvement is claimed.

All 12 tracked tasks are complete. Incremental caching currently covers pure
boundary decisions; compiler extraction and earlier inference conservatively
recompute from fresh facts. C# flow inventories remain unavailable, matching the
prior behavior. The full CallAtlas suite has 262 passes and the same 29 preexisting
failure IDs; all five new HTTP cases pass. See
[the exact upstream comparison](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/stages/http-upstream-tests.json).


<a id="source-evidence-engine-review-readme-md"></a>

## evidence/engine/review/README.md

Source: [evidence/engine/review/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/review/README.md). Historical record; retained rationale, methods and limitations.

### Standalone engine review against master

The reference is local `master` at
`fb54f5f789d94c6cbc5fe86bf20010edea815b33`. Review fixes stay on
`codex/standalone-graph-engine`; no merge or push is authorized.

#### Confirmed defects and fixes

- Canonical Java/C# identities nested on hydrate/export. Reuse only a verified
  canonical identity with the same complete owner; preserve native binding.
- Malformed portable paths and unsupported nested flow/architecture inventory
  versions were accepted. Validate them before hydration or comparison.
- Checkout names, nested TypeScript configurations, external compiler source
  aliases and escaped Java context paths changed exported metadata or identities.
  Normalize producer-owned path fields and folder-derived display names while
  preserving opaque compiler names, source literals and semantic applications.
- Java synthetic lambda ordinals changed after unrelated insertions. Export
  compiler-source fingerprints only for uniquely proven bytecode attribution.
  Ambiguous cases retain native IDs and an explicit warning. The user deferred
  exact ambiguous correlation to TASK-32.
- Unnamed TypeScript default classes, computed names and regex-containing
  anonymous bodies had unstable identities. Use compiler declaration ownership
  and parsed token boundaries, excluding trivia.
- Mutation or structural escape of the global Set constructor could manufacture
  observer invocation edges. Invalidate intrinsic proof on those operations.
- Custom ASP.NET HTTP metadata, endpoint providers and applied MVC conventions
  could invalidate inferred routes. Compiler-qualified unknown rewrites now
  block affected application owners. Actual .NET fixtures include negative
  controls and awaitable handler results; no runtime hooks are executed.
- A crashed engine could leave inherited stdout open or continuously emitting,
  hanging the desktop; prompt EOF could leak descendants. Observe leader exit
  independently, bound stream draining and reap the owned process group.
- Git index flags and external diff commands could conceal unrecorded upstream
  edits. Verify through a private index with flags cleared and raw diff enabled;
  the real upstream indexes remain byte-identical. The complete CallAtlas change
  is recorded in the pinned replacement patch and prior-checkpoint migration.

#### Verification

- Final workspace: 379 passed; 77 opt-in tests excluded from the default run.
- Strict workspace/all-target Clippy passed.
- Python: 58 project tests and 19 tooling tests passed.
- TypeScript adapter: 104 tests passed.
- Matching Java helper, standalone engine and desktop app distributions built.
- Pinned dependency-source verification passed for all three upstream checkouts.
- Modified-source CodeScene reviews scored 10; pre-commit safeguard passed.

- Real-language opt-in checks: 51 passed, covering Maven/Gradle ownership,
  duplicates, Lombok, Java flow/lambda/Spring/HTTP, C# policy/entries/ASP.NET/
  MediatR and the TypeScript production pipeline and boundaries.
- Final packaged/incremental/linked-source integration checks: 8 passed.
- All seven fixture graph files were refreshed with the final packaged engine.
  C# and MediatR files remain byte-identical. Maven/Gradle/Spring add explicit
  portable compiler-root provenance. TypeScript/mixed graphs update logical
  display paths and compiler-qualified React signatures plus their consistently
  translated symbol references. Node and edge counts remain unchanged.

- Desktop process protocol/roundtrip/cancellation/crash cleanup: all 8 passed.
- All seven workloads passed repeat, reversed-target and relocated-checkout
  byte equality; see [determinism results](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/review/determinism.json).

The [full native comparison](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/review/native-preservation.json) against the frozen
pre-review `b839ba3` distribution covers all seven workloads. Native identities,
complete edges, entries, flows, architecture, boundary facts, diagnostics and
content/membership freshness data are unchanged. Six TypeScript node signatures
now qualify React return types; corresponding portable keys change consistently.
Remaining differences are analyzer provenance and explicit Java root/folder
metadata. No unexplained binding or topology changes remain.

The original foundation equivalence against `master` is documented in the parent
engine evidence. Subsequent inference stages deliberately add reviewed possible
relationships; this review does not equate their output with pre-inference data.

The [matched performance report](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/review/performance.json) uses one warmup followed by
three alternating full-analysis runs per distribution on identical copied inputs.
Median runtime changed by +1.67% (TypeScript) and +1.45% (mixed), below the 5%
investigation threshold. Mixed sampled process-tree RSS median increased 6.69%;
run ranges overlap substantially. Sampling is a lower bound, can double-count
shared pages and is too variable here to claim a memory improvement.

Final whole-branch CodeScene review against `master` passed all 242 eligible
files out of 336 changed files, with no degradations. The pre-commit safeguard
passed all 48 eligible review-modified source files. Sonar was inactive because
this repository has no `.sonar/config.json`.

Reviewed fixes are committed as `5db7709`. Final evidence and task completion
are committed separately on the same feature branch. Local `master` remains at
the reference commit. No branch was merged or pushed.
One negative MediatR test initially failed during NuGet restore; its isolated
retry passed, including the expected compiler diagnostic. No analyzer failure
was observed on retry.

The complete CallAtlas suite has 262 passing tests and the same 29 failing test
names as the pre-review baseline. See [the exact comparison](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/review/callatlas-tests.json).
Those existing upstream failures are not claimed to pass. Private recurring
billing checks require unavailable private inputs. GPU performance experiments
are outside this engine review.


<a id="source-evidence-engine-stages-boundaries-md"></a>

## evidence/engine/stages/boundaries.md

Source: [evidence/engine/stages/boundaries.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/stages/boundaries.md). Historical record; retained rationale, methods and limitations.

### Navigation and HTTP boundary verification

The pre-inference fixture checkpoints are `dc32e17` and `9a6d5b6`, generated with
the frozen `b7207e2` distribution and its matching resources. Fixture source and
native graph changes are therefore separate from boundary inference changes.
Each project still owns only `.codegraph/graph.json`; Git retains these baselines.

The adapters emit compiler-owned route/request facts. The shared resolver joins
only proven App Router ownership or explicitly configured HTTP application
scopes. A destination is possible, not a proven synchronous invocation or proof
of deployment activation. Spring servlet prefixes, custom mapping configuration
and actual HTTP origin routing are outside this static subset. Unknown route
conditions stay potential competitors. No origin is inferred from build selection.

TypeScript tests exercise actual installed Next/TypeScript declarations, builtin
fetch, API shadowing, Pages Router exclusion, Link `as`, implicit HEAD, router
replacement/escapes, global-object structural replacement/escapes and deferred
class initialization. Real Rust acceptance checks graph/flow target agreement and
preservation of every preexisting native edge. JSX links add no synthetic flow
call sites. Next configuration and middleware/proxy presence conservatively block
routing; both missing and present files participate in source verification.

Shared tests cover same-owner static precedence, cross-application ambiguity,
case-insensitive ASP.NET collisions, constrained routes, unsupported catchalls,
unknown methods/order, config overlap rejection, canonical relocation, nullable
blocker identities and desktop SQLite round trips. The production Rune flow
inspector renders the explicit HTTP kind.

The packaged engine preserves all nodes/native edges/architecture facts and full
flow payloads after removing only the newly owned boundary derivations. Counts:
TypeScript 75 nodes/54 edges (4 boundary edges), Spring 29/38, ASP.NET/MediatR
49/11, mixed 179/120 (6 boundary edges). The mixed additions are Next GET,
Spring health, ASP.NET Ready, Link-to-About, router-to-About and router-to-User.
Common `/api/items` destinations and constrained/case/order collisions stay
unresolved. Complete preservation counts are in `boundary-preservation.json`.

Repeat/reordered/relocated packaged analyses pass for TypeScript, Spring,
MediatR and mixed workloads (`boundary-determinism.json`). The standalone
engine and signed desktop app bundle were rebuilt with matching resources.
All 98 TypeScript compiler tests, 56 project Python tests, 19 tools tests,
workspace all-target tests, strict workspace Clippy and actual language/flow/UI
acceptance checks pass. The worker-count test required process-table access;
its sandbox-denied run was rerun successfully with that access.

The complete upstream CallAtlas suite has 262 passing tests and the identical
29 preexisting failing test IDs. All five new HTTP cases pass; there are no new
or removed failures. `http-upstream-tests.json` records the exact comparison.
Existing large upstream file Code Health scores are unchanged; new implementation
files score 10.0 and the pre-commit safeguard passes.

Three isolated alternating baseline/candidate measurements used the frozen
`b7207e2` distribution and matching packaged Stage5 resources on unchanged source
fixtures. Median runtime deltas: TypeScript +1.45%, Spring +2.37%, C# -0.58%.
Sampled process-tree RSS medians: TypeScript 342,491,136→345,964,544 bytes,
Spring 259,309,568→271,532,032, C# 293,371,904→294,092,800. Sampling completed
without errors. These small-fixture measurements do not establish a speedup;
no runtime or sampled-memory increase exceeds 5%. Raw runs are in the three
`boundary-*-performance.json` reports. Process-tree RSS may count shared pages
more than once and is not an operating-system peak-memory guarantee.


<a id="source-evidence-engine-stages-incremental-md"></a>

## evidence/engine/stages/incremental.md

Source: [evidence/engine/stages/incremental.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/stages/incremental.md). Historical record; retained rationale, methods and limitations.

### Incremental inference verification

Baseline: verified boundary checkpoint `3f8b652` and its frozen matching engine
and resources. The incremental pass caches pure boundary matching decisions;
compiler extraction and the earlier language-specific inference passes remain
fresh/full. Cached graph edges, source evidence, flow trees and node indexes are
never replayed. This is deliberately narrower than caching compiler analysis.

Dependency keys include the automatic implementation digest, format version,
current use, explicit application configuration, toolchain and complete potential
candidate/blocker membership. Current graph validation proves symbol support;
the use and route facts carry those exact identities. An added candidate changes
a negative dependency key. Unrelated node changes do not invalidate every use.
Fresh flow attachment is outside the cached decision and always rechecks the
actual current source site.

The cache is read-only until graph publication succeeds. The existing project
lock covers both graph publication and the subsequent current-row transaction.
Missing/incompatible/malformed cache data falls back to full inference; SQLite
ownership markers and regular-file checks protect unrelated files. Corrupt or
unowned cache files are preserved with volatile warnings. Cache errors never
change canonical graph diagnostics or successful publication into failure.

Verification:

- All seven canonical project graphs are byte-identical to `3f8b652`: Maven,
  Gradle, collision C#, TypeScript/Next, Spring, MediatR/ASP.NET and mixed.
  `incremental-corpus.json` records the exact SHA-256 values.
- Packaged repeated/reordered/relocated TypeScript and mixed runs match byte for
  byte. Cold runs recompute 13 decisions; warm runs reuse all 13 with zero misses.
  `incremental-determinism.json` records the checks. Each temporary project owns
  one graph path; comparisons do not write history snapshots.
- Fourteen real-SQLite tests cover positive/negative dependencies, unrelated reuse,
  blocker changes, support deletion, index rebinding, stale derivation cleanup,
  format/code invalidation, malformed winners/misses and key-bound checksums,
  current-row replacement, unowned/corrupt files, database/sidecar links, immutable
  prepublication reads, unowned WAL preservation and bounded locked-cache writes.
- Three real CLI tests pass against the latest engine binary. Cache/full bytes
  agree after source/route additions, edits and deletions, client rewiring, Next
  config blockers and observer deregistration. Completed fingerprints remain
  fresh after publication. Compile failure, cancellation and a real graph
  publication failure preserve both graph and previous cache bytes.
- The final workspace all-target suite and strict workspace Clippy pass. Modified
  source/test files score CodeScene 10.0 (reexports have no scored findings).
  The signed desktop app and relocatable standalone distribution were rebuilt
  with matching analyzer resources. Upstream source did not change in this stage;
  the previously documented 29 CallAtlas baseline failures remain the limitation.

Initial isolated performance runs are retained in
`incremental-initial-*-performance.json`. Their JVM timing/RSS variation prompted
an additional controlled round: the old engine sees its original pre-cache file
layout, while the candidate retains its warm derived cache. Cache/sidecar backups
exist only in process memory between paired runs; graph files remain at their
single current path. The corrected reports describe this protocol explicitly.
Sampled process-tree RSS is a lower bound and can count shared pages repeatedly;
short JVM processes can show substantial sampling/heap variation. These fixtures
do not establish a general memory or speed improvement.

Controlled paired median runtime (baseline → candidate): TypeScript
0.6448→0.6355s (-1.43%), Spring 2.6323→2.6136s (-0.71%), C#
3.3228→3.3183s (-0.14%), mixed 15.1471→15.0642s (-0.55%). No runtime
regression above 5% was observed. Sampled RSS medians respectively were
348,471,296→346,570,752; 303,808,512→213,942,272;
293,502,976→293,552,128; 373,571,584→352,468,992 bytes. Sampling completed
without errors. The very different Spring RSS observations between rounds
illustrate JVM/sampling variability, not a demonstrated cache memory improvement.
The cache avoids repeated matching decisions, not compiler execution; these small
workloads do not establish a meaningful speedup.


<a id="source-evidence-engine-stages-java-equality-inference-2026-11-11-md"></a>

## evidence/engine/stages/java-equality-inference-2026-11-11.md

Source: [evidence/engine/stages/java-equality-inference-2026-11-11.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/engine/stages/java-equality-inference-2026-11-11.md). Historical record; retained rationale, methods and limitations.

### Java equality inference verification — 2026-11-11

TASK-35 disables java-callgraph2 type/value inference only for exact
`equals(java.lang.Object)` and `hashCode()` signatures. The analyzer retains
their declarations and bytecode call targets. Other methods, including overloads,
retain full inference and the existing strict completeness checks.

#### Reproduction and regression checks

- Before the change, the exact-signature regression failed because an
  `equals(Object)` caller still had an inferred concrete receiver target.
- After rebuilding the adapter, all 11 real-JVM context tests passed, including
  a 40-field Lombok class, authored equality overrides with direct calls,
  similarly named methods, overloads, and ordinary concrete receiver inference.
  New inference regressions have a 30-second analyzer deadline.
- All 19 focused Java acceptance tests passed: flows (5), HTTP (2), lambda
  correlation (4), lambda identity (3), Lombok (2), and Spring events (3).
- All 10 focused process tests passed, including cancellation, short deadlines,
  descendant cleanup, worker wait/finish cleanup, cancellation precedence, and
  a bounded 4,000-byte stderr tail with a split UTF-8 character.
- The cancellation protocol test exposed an unread-output-pipe deadlock after
  contextual errors became longer. Its harness now drains both pipes while
  waiting and verifies the final cancellation event. All four protocol tests
  pass, including preservation of the previous artifact and descendant cleanup.

#### Private reactor verification

The existing Recurring-billing checkout was copied with APFS copy-on-write to a
temporary directory, including its local inputs. Analysis selected the copied
root `pom.xml` explicitly. Automatic discovery also found a separate nested
worktree with unavailable TypeScript dependencies; that unrelated target was not
part of the failed Java reactor analysis.

Using the rebuilt development engine/resources, the complete Maven reactor
finished successfully in 220.04 seconds and atomically published a canonical
graph containing 10,827 declarations and 17,331 relationships. All 13 Java
contexts passed the existing strict completeness/snapshot checks. Publication
digest: `d07a117eff1246bed94c35b469cffe698b0f7f206e3a0e9526147396ff3ea0f5`.
The new equality inference diagnostic is present in the published graph.

The rebuilt adapter also analyzed the original saved failing context directly:
`analysisComplete=true`, `engineSucceeded=true`, and
`frameSnapshotLimitMethods=0`. Its retained status is under
`/private/tmp/codegraph-35-saved-context/analysis-status.properties`.

#### Build and quality checks

- Desktop all-targets check and tests passed.
- Engine all-targets check and tests passed after correcting the protocol harness.
- Strict workspace Clippy passed.
- `python3.13 tools/package.py --engine-only` passed.
- Independent final review found no correctness blockers.
- All seven modified source/test files scored 10.0 in CodeScene. The safeguard
  passed with seven eligible files checked and no regressions.
- SonarMCP is inactive: this project has no `.sonar/config.json`.

The exact debug `cargo test --workspace --all-targets -- --include-ignored`
command failed in three existing canvas benchmarks that explicitly require
release mode; two additionally require a copied profiling workspace. A separate
functional run excludes only those benchmarks. Its initial attempt reached the
private duplicate-declarations acceptance test and requested
`CODEGRAPH_RECURRING_BILLING`; the final run supplies the copied reactor path.
That private acceptance test passed. All three canvas benchmarks also passed in
their supported release profile with a copied saved workspace and Metal access:

```sh
CODEGRAPH_DATA_DIR=/absolute/path/to/copied/workspace \
  cargo test --release --locked -p codegraph --bin codegraph canvas_ \
  -- --ignored --test-threads=1
```

The functional workspace run completed all desktop/core targets and 459 tests
successfully, then stopped at the engine's compiled-private-reactor test because
`CODEGRAPH_COMPILED_METADATA` was unset. A separate real Maven `test-compile`
exported that manifest from the copied reactor. Supplying both private fixture
variables, the complete engine all-targets suite with `--include-ignored` passed
all 67 tests, including the compiled-reactor, linked-source, cancellation, and
artifact-preservation acceptance tests. The remaining workspace test coverage
therefore passed across these runs; the raw debug command's three release-only
benchmark failures were resolved by the separate supported-profile run above.

Metadata export and final engine acceptance logs:
`/tmp/codegraph-35-compiled-metadata.log` and
`/tmp/codegraph-35-engine-acceptance-final.log`. The manifest is retained at
`/tmp/codegraph-35-compiled-final.json`; its source/build paths reference the
temporary reactor copy and should be regenerated if that copy is removed.

Investigation/build/test logs are retained under `/tmp/codegraph-35-*.log`,
`/tmp/codegraph-process-*.log`, and
`/private/tmp/codegraph-equality-java-*.log`. No private source or generated graph
payload is checked into this repository.


<a id="source-evidence-graph-comparison-readme-md"></a>

## evidence/graph-comparison/README.md

Source: [evidence/graph-comparison/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/graph-comparison/README.md). Historical record; retained rationale, methods and limitations.

### Git graph comparison acceptance

TASK-58 adds the version-1 `compare` query to both public CLI names and the
`compare_graphs` MCP tool. [The CLI contract](<../../../docs/cli-queries.md#git-graph-comparison>)
documents arguments, field-level records, scope, snapshot binding and page limits.

#### Behavioral verification

- Core tests exercise complete graph sections, opaque extensions, unordered
  collections versus ordered flow, certainty/provenance variants, exact identity,
  explicit null versus absence and graph/artifact namespace collisions.
- Temporary real Git repositories exercise immutable commit resolution, nested
  project paths, exact-byte digests and a missing promisor blob. Missing partial
  clone objects fail without fetching or creating `FETCH_HEAD`.
- Engine process acceptance covers stale saved working graphs, two committed
  revisions without a working artifact, branch movement with identical graph
  bytes, session parity, complete pagination, byte limits and structured errors.
- Real MCP stdio acceptance compares full CLI envelopes and pages and verifies
  committed freshness, errors, immutable input files and no cache writes.
- Separate real MCP cancellation/disconnect tests hold the actual CLI in a Git
  read, then verify both CLI and Git PIDs terminate and saved bytes remain intact.

The legacy standalone comparison retains its original DTO and exit behavior.
Comparison semantics, artifact retrieval, CLI dispatch, transport framing and MCP
wire validation have separate owners. Independent review findings concerning
extension namespaces and partial-clone fetching were fixed and regression tested.

#### Large graph measurement

[Raw measurements](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/graph-comparison/latency-memory.json) use a retained compiler-produced graph
with 10,827 nodes and 17,331 relationships. The baseline is 113,628,316 bytes;
the copied target changes one node label and adds one diagnostic. The source
project was not modified or analyzed. Raw inputs remain ignored under
`target/verification/task58-large-comparison/`, with the baseline in that temporary
repository's Git history and the target in its working graph. Private source
content is not included in this evidence directory.

Three sequential release `ccnavigator query` processes returned two records in a
7,492-byte envelope. Median elapsed time was 4.3998 seconds; maximum observed
resident memory was 4,017,881,088 bytes (about 3.74 GiB). Timing includes Git blob
retrieval, artifact validation, freshness observation, comparison and JSON output.
Filesystem/OS caches were uncontrolled. These controlled edits measure the
comparison machinery, not the size or cost of an actual historical change.

These are the initial implementation's measurements. Its canonical comparison
copies were subsequently removed as described below. Pages remain stateless and
re-read and compare their endpoints; a small response does not imply a small
working set.

#### Memory optimization (2026-11-13)

[Before/after samples](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/graph-comparison/memory-optimization.json) remeasure the same retained graph
pair and an additional case that changes one function diagnostic. The baseline
executable is retained alongside the inputs and full response envelopes under
`target/verification/task58-memory-optimization/`. Each case was measured with
three sequential release processes per executable using macOS `time -l`, after
full verification completed and was idle. The after executable is the verified
packaged CLI.

| Case | Before peak RSS | After peak RSS | Reduction | Before median | After median |
| --- | ---: | ---: | ---: | ---: | ---: |
| Node label and graph diagnostic | 3.73 GiB | 2.07 GiB | 44.4% | 4.69 s | 2.91 s |
| Same pair plus one function diagnostic | 5.18 GiB | 2.07 GiB | 60.0% | 5.11 s | 2.95 s |

Peak figures are the maximum across the three samples, and times are medians.
Filesystem caches and other machine activity were uncontrolled. The earlier
instrumented investigation's flow-change RSS is not used as this table's baseline;
both columns here use uninstrumented release CLI processes on identical files.

The CLI consumes loaded artifact graphs for canonicalization instead of retaining
two additional full graph trees. Graph and provenance validation deserialize
typed values from borrowed JSON without intermediate raw JSON copies. Collection
groups contain borrowed references, and only emitted change payloads become owned
JSON. The public borrowed comparison API still leaves its inputs intact, clones
only graphs when necessary, and borrows opaque artifact extensions throughout.

Every complete parsed CLI envelope in both retained cases matched the pre-change
response: no fields were excluded from equality, including digests, comparison
binding, freshness, record ordering and summaries. Core regressions additionally
cover singleton/duplicate function and file groups, unordered collection ordering,
ordered flow and opaque extension arrays, large unchanged opaque extensions and
invalid artifacts. Existing Git, pagination, byte-budget, session and MCP
acceptance remain required.

Independent [old/new parity evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/graph-comparison/memory-parity.json) covers 12 temporary
real-Git cases: unchanged data, metadata, edge variants, flow diagnostics, ordered
flow, duplicate inventories, opaque/node fields, unordered collections and invalid
schema/reference/type/provenance inputs. Complete query envelopes on every page
and legacy stdout, stderr and exit codes match. Graph files remain byte-identical.

The remaining roughly 2.07 GiB peak includes the two expanded JSON trees and
validation/sorting transients. Compact typed flow storage was deliberately left
outside this optimization. No graph schema, query response or comparison format
version changed.

##### Supplemental upstream test result

The separate CallAtlas `dotnet test --configuration Release` run built all three
projects for .NET 11 and reported 262 passed and 29 failed tests. Independent
source/TRX review classified all 29 first failure sites as unqualified symbol-ID
assertions and lookups, such as `IShape::Draw()` and
`X::F()::Inner#local`, while the recorded assembly-identity patch emits qualified
IDs. This task changes only the three project target frameworks within that
upstream patch; it does not change its analyzer logic or upstream assertions.
Twenty-six failures concern snapshot assembly qualification and three concern
delegate assembly qualification. No distinct .NET 11-specific assertion failure
was found. This classification does not establish whether later assertions would
pass after correcting IDs, and no .NET 10 control run was performed.
This supplemental suite is outside the configured workspace verification gate
and is not reported as passing. The complete log, TRX and extracted failures are
retained under
`target/verification/task58-memory-optimization/upstream-tests/` (the command log
is `target/verification/task58-memory-optimization/upstream-net11.log`).

#### Memory optimization verification

The final run at `target/verification/task58-memory-final-rerun/` passed all
43 stages outside the macOS sandbox using the installed .NET SDK 11.0.100/runtime
11 and Graphviz 16.1.1. It passed 574 ordinary Rust tests, all 667 release tests
(zero ignored), 218 Python tests, 112 TypeScript tests, CPU/GPU and storage
benchmarks, app/engine packaging and signature checks, installed CLI acceptance,
independent MCP installation and integration journeys, CodeScene and cleanup.
The analyzer, generated .NET fixtures, build/package paths and release CI target
.NET 11. The upstream project edits are captured in the checked-in patch; its
previous patch is retained as a verified bootstrap migration checkpoint.

Earlier attempts are retained separately: the initial memory run failed preflight
against outdated version requirements, the first latest-toolchain run was
explicitly interrupted before simplifying the remaining .NET 10 fixtures, and
`task58-memory-final-full` passed 666 release tests but failed one fixture's
`dotnet restore` assertion. That assertion omitted subprocess output. An exact
standalone recreation restored successfully, so its original cause remains
unknown. The assertion now includes status/stdout/stderr, without changing restore
requirements or adding retries; both MediatR identity tests passed in the final
full rerun. Both private Maven regressions passed; registry DNS retries delayed
dependency installation in the earlier attempt.

Every affected source file was reviewed with CodeScene at 10.0; the independent
review found no remaining actionable concerns. The pre-commit safeguard passed
for all 17 eligible files, and the final full CodeScene stage passed. SonarMCP
remains inactive because `.sonar/config.json` is absent. The supplemental upstream
test failures above are reported separately and are not claimed as passing.

#### Initial delivery verification

The full run uses the existing compiler fixture modules and private Maven
checkout, and snapshots the saved application workspace before benchmarking.
Evidence is under `target/verification/task58-full/`. An earlier preflight-only
attempt under `full-w5n2id84` failed because the required input environment
variables were unset; it did not establish acceptance results.

All 52 stages passed: 568 ordinary Rust tests, 661 release tests including all
ignored compiler/real-process acceptance and CPU/GPU benchmarks, 215 Python tests,
112 TypeScript tests, storage benchmark, app/engine packaging and signature,
installed CLI acceptance, independently installed MCP and integration journeys,
CodeScene and artifact cleanup. The private UI dependency installation emitted
registry DNS warnings and delayed both private builds; both builds and their
acceptance tests ultimately passed. No exception was needed.

Every affected source file was reviewed with CodeScene; findings were resolved
to 10.0. The pre-commit safeguard and full CodeScene stage passed without
regressions. SonarMCP is inactive because this checkout has no `.sonar/config.json`.


<a id="source-evidence-graph-contract-readme-md"></a>

## evidence/graph-contract/README.md

Source: [evidence/graph-contract/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/graph-contract/README.md). Historical record; retained rationale, methods and limitations.

### Portable graph contract hardening — 2026-11-11

TASK-39 implements the approved CodeGraph-only contract plan. See
[consumer contract](<../../../docs/graph-contract.md>) and
[Draft 2020-12 schema](<../../../docs/graph.schema.json>).

#### Implementation and review log

1. Recorded Draft DRAFT-1, promoted to TASK-39 and activated with the approved plan.
2. Characterized legacy relocation, ordering, references, evidence and publication;
   introduced an explicit portable graph model, typed transformations, domain
   validation with a shared endpoint index, and structure-scoped canonical ordering.
3. Added compact serialization and opaque encoded artifact buffers shared by digest
   computation and atomic publication. Legacy loading does not rewrite files.
4. Added portable observed-input provenance, content verification and CLI reporting;
   removed machine-specific compiler context/version-report paths. Added independent
   schema/consumer tools and documentation.
5. Independent review found and resolved reserved envelope extension collisions,
   raw-byte publication bypass, manifest ordering idempotence, nullable metadata,
   and unavailable-root classification. CodeScene findings drove domain helper
   extraction; all reviewed source files reached 10.0 and the safeguard passed.

#### Passing verification

- `cargo check --locked -p codegraph --all-targets`.
- `cargo test --locked -p codegraph --all-targets` (normal ignored-test policy).
  An initial concurrent run had a process-test PID parsing race; the complete rerun passed.
- `cargo test --locked -p codegraph-core --all-targets`: 55 passed.
- `cargo test --locked -p codegraph-engine --all-targets`.
- `cargo test --locked -p codegraph-engine --test verify`: 2 passed.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`.
- `python3.13 tools/package.py --engine-only`.
- Both real Java and C# `linked_sources` ignored acceptance tests passed when run
  outside the sandbox to permit Maven/NuGet cache locks and compiler host pipes.
- Seven independent Python schema/consumer tests passed using test-only
  `jsonschema>=4.18,<5` in a temporary virtual environment.
- CodeScene file reviews: all modified/new source files 10.0. Final pre-commit
  safeguard passed with 20 eligible files checked. No Sonar configuration exists.

#### Real TypeScript consumer acceptance

Analyzed a temporary copy of the existing TypeScript corpus, with explicit logical
roots for compiler dependencies and observed ancestor package-manager inputs.
The retained mixed-language fixture (179 nodes, 120 edges) and fresh production
artifact (75 nodes, 54 edges, 29 evidence records, 289 input records) passed the
published schema and independent JSON consumer.

Fresh output was 251,159 bytes, compact JSON plus one newline. Repeated production
analysis produced identical bytes. Publication, saved-file and verification digests
agreed (`21bddf1b2b6e00bc74b50d7d3b438e05262efe6eb12d822cb6d78cd2679b1517`).
Full project-file mtime/size/SHA256 snapshots showed no verification writes.
A same-size source edit with its original timestamp restored produced exit 1 and
`stale`; restoring source bytes returned exit 0 and `match`, without changing the
artifact. The packaged engine also verified the artifact successfully.

Without a logical root covering observed ancestor inputs, verification correctly
reported `unverifiable`. This is deliberate: observed-input provenance does not
certify inputs that cannot be mapped portably.

#### Full-suite limitations

`cargo test --workspace --all-targets -- --include-ignored` was run and failed:

- Three canvas performance tests explicitly require `--release` rather than the
  documented debug invocation.
- The detached-process cancellation acceptance test reported a surviving child
  under the sandbox; it is explicitly marked as requiring external process access.

A separate engine run with ignored tests reached the unavailable private recurring
billing fixture (`Set CODEGRAPH_COMPILED_METADATA`). Skipping that fixture exposed
sandbox Maven lock/compiler host pipe failures in linked-source tests; rerunning
those two tests with escalation passed. The entire ignored workspace suite is not
claimed passing. No unrelated performance/process behavior was changed.

The user explicitly accepted the documented full-suite limitations and requested
TASK-39 completion and a commit. The acceptance applies to the release-only
performance tests, sandbox process-cleanup failure, and unavailable private
compiler fixture described above; those checks are not claimed passing.


<a id="source-evidence-mcp-readme-md"></a>

## evidence/mcp/README.md

Source: [evidence/mcp/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/mcp/README.md). Historical record; retained rationale, methods and limitations.

### CLI-only MCP acceptance

The separate `codecallnavigator-mcp` package uses the official `rmcp` SDK and has
no core, engine or desktop dependency, including test dependencies. Runtime code
accesses graph/source information only through the configured CLI executable.
See [installation and public tools](<../../../docs/mcp.md>).

#### Process acceptance

Package integration tests speak real MCP JSON-RPC over stdio, without importing
application libraries. A real TypeScript compiler fixture exercises explicit
refresh, same-named symbol ambiguity, details/source/function flow/neighbors,
bounded selections and exact response comparison with direct CLI queries. It
also covers linked-root explorer access before analysis, missing graphs/symbols,
live versus analyzed source, staleness and generation mismatches.

Real Gradle cancellation/disconnect fixtures first publish a compiler-produced
graph, then stop a wrapper and its child during another refresh. Tests check that
the previous artifact remains byte-identical, cancellation emits no late MCP
response, and another generation-bound query succeeds. Narrow fake CLI fixtures
cover incompatible capabilities, malformed response metadata and hung queries.

Focused process regressions cover blocked writes, dropped request futures,
stderr backpressure before the CLI hello and independently grouped descendants.
Unexpected CLI crashes can orphan processes before ownership is observable; that
case reports unverified descendant cleanup instead of promising complete recovery.

#### Independent installation

`tools/package_mcp.py` creates the MCP's own Cargo archive, extracts it outside
the repository, builds/installs it into a temporary prefix, verifies its native
signature/help, and runs real stdio against the separately installed CLI. The
result contains only the MCP binary, guide and license notices; it bundles no
engine, analyzers or desktop resources. The source `.crate` is retained alongside
the binary archive.

An initial check exposed stale Cargo reuse with normalized archive timestamps;
installation now overrides both Cargo build directories with a fresh temporary
directory. Complete installation/package/smoke passed afterward. Boundary checks
also reject transitive test/target dependencies on application packages and
source inclusion in runtime/tests, while permitting test fixture filesystem use.

Full verification runs under `target/verification/task50-full-20261112-r3/`.
The earlier `task50-full-20261112` run passed 545 ordinary Rust tests, then stopped
on an existing verification fixture's 300 ms interpreter-startup timeout before
its readiness PID existed. The same failure reproduced in its focused suite.
The fixture now allows one second while retaining the three-second cleanup bound,
30-second hanging process and both stopped-PID assertions; all 16 focused tests
pass. Original failure evidence remains intact.

The second run, `task50-full-final-20261112`, caught a separate readiness race:
a desktop fake CLI exposed a newly created PID file before writing its contents.
Test fixtures now publish completed PID files atomically in both desktop and MCP
process tests and in the Python quality fixture. Seven focused desktop query
tests and the 16 quality tests pass, and independent review confirms the original
cancellation/restart/process-death assertions remain intact.

All 44 stages passed in the final run: 546 ordinary Rust tests, 636 release tests
including ignored acceptance, 164 Python project tests, 26 Python tool tests and
112 TypeScript tests. Storage targets, CPU/GPU benchmarks, app/engine packaging,
signature checks and MCP independent archive installation/stdio smoke passed.
CodeScene reviewed all 23 eligible changed source files without findings; its
23/23 safeguard passed without regressions. Final cleanup passed as well.

The Backlog task records the chronological action log. Full checks include the
independent installation stage and use an isolated profiling copy; Sonar is
inactive because the project has no `.sonar/config.json`.


<a id="source-evidence-queries-readme-md"></a>

## evidence/queries/README.md

Source: [evidence/queries/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/queries/README.md). Historical record; retained rationale, methods and limitations.

### CLI query measurements

Task: TASK-48. Measurements taken on 12 November 2026 with an optimized engine,
using the existing 113,628,316-byte compiler-produced Java project artifact.
The measured project and its artifact were read only. No live desktop store was
benchmarked. Raw durations and graph digests are in `billing-latency.json`.

| Operation | One-shot median | Persistent session median |
| --- | ---: | ---: |
| Status | 1,860 ms | 233 ms |
| Search | 1,864 ms | 234 ms |
| Overview selection | 1,891 ms | 249 ms |
| Complete render snapshot | 4,425 ms | 3,275 ms |

Each operation has three samples. Timings include process work, pipe transfer
and Python JSON decoding. Filesystem caches were uncontrolled, so these are
not cold-disk benchmarks. The first session status took 1,945 ms to load the
generation; subsequent status requests took 233 and 230 ms. Full snapshots
transfer all analyzed data and are intended for generation changes rather than
interactive queries. Session reuse reduces search and selection latency by
approximately eightfold on this graph, but does not promise sub-100 ms results.

The CLI session hashes the current artifact bytes for every request, rejects a
different expected generation, and checks observed source inputs again even
when reusing the parsed graph. Artifact identity is not inferred from timestamps.

Reproduce against a prepared project without running analysis:

```sh
cargo build --locked --release -p codecallnavigator-engine --bin codecallnavigator-engine
python3 tools/query_benchmark.py --engine target/release/codecallnavigator-engine \
  --project /path/to/prepared/project --output /tmp/query-latency.json --repeats 3
```

Behavioral regressions cover session freshness after source edits, generation
replacement, generation-bound rejection, and orderly EOF shutdown in
`crates/codecallnavigator-engine/tests/query_session.rs`. Existing verifier
integration tests continue to cover mixed stale/unverifiable inputs and
read-only behavior.

#### Task 48 verification

The full run at `target/verification/task48-full-20261112/` passed all runtime
stages: 502 ordinary Rust tests, 587 release-mode Rust tests including ignored
acceptance/benchmarks, 155 project Python tests, 26 tool tests and 112 TypeScript
tests; storage benchmarking, packaging and strict signatures also passed.

The original final quality stage failed because declaration-only Rust modules
returned a null CodeScene score. The runner now records their explicit
non-applicability using a conservative declaration-only grammar; executable and
unknown source still require a numeric score. This tooling-only correction
passed independent review, 16 quality tests and 19 orchestrator tests. A test
fixture builder was also split by responsibility after a Large Method finding;
its six query tests and Clippy passed again. Production code was unchanged after
the full runtime run.

The affected quality stage was rerun over every current changed source:
`35-codescene-rerun.json` reports success and 19/19 eligible files checked with no
regressions. The original failed report is preserved alongside the recovery
report rather than overwritten. No verification exceptions were accepted.
