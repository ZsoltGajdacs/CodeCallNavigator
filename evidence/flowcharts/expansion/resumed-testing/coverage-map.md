# Resumed testing coverage map

Read-only audit on 2026-11-15 after the `d792304` testing-strategy change. This
records the current source and inventory, not a passing verification of the
in-progress Unit 3 implementation. No builds or tests were run for this audit.
Only this evidence document was written.

## Capability-test migration

The two engine libtest identities remain unchanged:

- `analyzers::typescript::tests::stale_missing_and_future_typescript_adapters_require_matching_resources`
- `analyzers::typescript::tests::current_typescript_adapter_is_accepted_with_unchanged_prepare_contract`

Previously, `prepare_adapter` created a temporary `analyze.js` that fabricated
successful compiler metadata, wrote that metadata to the requested output, and
kept a Node process alive on stdin. Each invalid-version combination launched a
replacement worker and called production `prepare`. This was engine integration
using a business-result double, not a successful real TypeScript journey or a
necessary protocol fault probe. Its unique intended assertion was resource-version
policy; the synthetic configuration and worker provided no additional compiler
evidence.

The replacement tests call production `validate_capabilities` with explicit
in-memory JSON. Production `prepare` calls that same function immediately after
decoding actual worker metadata. There is one policy implementation, no copied
predicate and no replacement compiler/worker. The tests are now ordinary,
unignored engine **unit** tests with an empty requirements list in
`tools/verification/test-inventory.json`.

| Original defect/assertion | Retained direct proof | Boundary proof retained |
| --- | --- | --- |
| Missing, stale `"6"`, future `"8"`, numeric `7`, or null adapter accepted | The first identity supplies each value with valid `boundary_rules:"1"`; each must return an error containing rebuild guidance. Removing the adapter-version guard makes `expect_err` fail. | Real CLI TypeScript analysis consumes actual metadata through `prepare`, then publishes/query-checks the resulting artifact. |
| Matching adapter rejected | The second identity requires `adapter:"7", boundary_rules:"1"` to succeed. | Real current adapter and compiler are exercised by the public CLI journeys and Node compiler fixtures. |
| Boundary capability weakened | The second identity additionally rejects missing, null, numeric `1`, and unsupported string `"2"` boundary versions. | Existing TypeScript preparation/configuration/compiler fixtures remain registered as engine integration. |

Observed original red evidence is retained at
`/private/tmp/codecallnavigator-task22.7/target/unit2c-verification/capability-red.log`:
the stale/missing test failed with **“Stale/unknown adapter must be rejected”**,
0 passed / 1 failed. Its hash is recorded in
`evidence/flowcharts/expansion/unit2c/report-index.json` as
`780d030a943fa942061e2edf1d07d7ce95f0790d97c53dfbff2b868c400cac72`.
The new unit assertions exercise the precise missing version-check failure,
rather than relying on matching test names. After the initial preparation audit,
the parent ran the engine unit category. Independently inspected
`target/verification/task22-resumed-engine-unit/04-unit-test-rust.log` contains
both exact identities passing, unignored. All six category stages passed in
`results.json`, including category cleanup. This is an independently inspected
parent-run green, not an additional tester execution or mutation trial.

Historical cost, not a current benchmark: the old two-case release run recorded
1.04 seconds of libtest execution; its independent batch recorded 5.328 seconds
wall time including Cargo work. These observations are in the retained Unit 2C
independent report and `independent-capability-release.log`. There is no measured
isolated replacement-pair timing, so this audit claims no measured speedup. The
new category report separates compilation 10.636s, preparation 0.958s, Rust test
stage 2.351s, Node unit stage 1.012s and cleanup 1.078s (prerequisites 0.902s).
Those whole-stage figures include other selected tests and cannot be compared
directly with the old two-case execution figure.

## Retained coverage and classification

| Coverage | Component/category | Requirements and reason for this layer |
| --- | --- | --- |
| The two capability-policy identities above | Engine / unit | In-memory JSON and the actual production validator; no Node/compiler/process requirement. |
| `real_typescript_facts_survive_stable_ids_cached_queries_and_excluded_targets` in `engine/tests/typescript_facts.rs` | Engine / acceptance | Inventory requires Node and TypeScript; Cargo builds the actual CLI. A cheaper layer cannot prove analyze → canonical publication → public cached query. |
| `real_typescript_analysis_is_queryable_without_reanalysis` in `engine/tests/query.rs` | Engine / acceptance | Inventory requires CLI, Node and TypeScript. Retains the existing complete query journey. |
| `tools/typescript/adapter.test.js` and `flow-export.test.js` real source/compiler fixtures | Engine / integration | Node and installed TypeScript. Real compiler declaration ownership, overloads, configuration inputs and source evaluation need compiler boundaries. |
| Shared invocation-fact/legacy serialization checks | Core / unit | Explicit IR/artifact fixture inputs prove strict facts, uncertainty and exact legacy bytes; fixture loading does not make these integration tests. |

The new CLI journey analyzes the actual fixture using
`CARGO_BIN_EXE_codecallnavigator-engine`; it asserts synchronous execution,
isolated/direct invocation and stable owned IDs, prunes a real target while
preserving invocation facts, deletes `main.ts`, and queries with PATH restricted
to `/usr/bin:/bin`. It checks cached Unicode source, returned facts and unchanged
artifact bytes. These are real outputs, not fabricated successful services.

Retained actual compiler fixture identities include:

- `type errors reject analysis and config preparation reports engine inputs`
- `compiler bindings retain anonymous callback ownership and overload implementations`
- `runtime import aliases resolve one authored overload implementation`
- `simple ordinary function calls prove only their isolated final invocation`
- `generator parameter defaults are a synchronous prefix before a deferred body`

Protocol/process failure probes elsewhere are not replaced or presented as real
successful journeys by this migration. No test is removed solely to meet a time
aspiration, and no correctness/recovery timeout is shortened.

## Commands and readiness

After freeze, use the shared category execution paths, without also repeating the
same ordinary tests in release:

```sh
./verify-unit.sh --component engine
./verify-integration.sh --component engine
./verify-acceptance.sh --component engine
./verify-unit.sh --component core
./verify-full.sh --dry-run
```

The first selects the two migrated policy tests; integration retains real Node
compiler fixtures; acceptance retains both public CLI journeys. Exact selected
registrations, requirements, builds, setup, execution and cleanup must be read
from the post-freeze category reports. Full remains required for the overall
cross-component/analyzer change and is owned by the parent task.

At this audit checkpoint, unit-engine, acceptance-engine and full **dry runs were
unavailable**: inventory discovery referenced the not-yet-created
`crates/codecallnavigator-core/tests/flow_expanded_contract/identity.rs` during
the active Unit 3 test split. Each returned exit 2 before any build/execution or
evidence creation. This was a transient preparation observation, not a waived gate;
the later parent engine-unit run successfully selected and executed the policy
tests. Repeat the remaining selection checks after final freeze.
`verify-unit.sh --help` successfully confirms
component/suite/dry-run/profile/timeout/keep-artifacts options.

Current shell inspection (no secrets or private project contents read):

- Cargo/Rust, Node, Java/javac, dotnet, Maven, Graphviz, codesign, xcrun, lsof and
  cs-mcp resolve on PATH. The local verification Python exists. Tool presence is
  not toolchain/version/native-access preflight success.
- `CODECALLNAVIGATOR_TS_NODE_MODULES`, `CODECALLNAVIGATOR_RECURRING_BILLING` and
  `CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE` are unset; no conflicting
  `CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT` is set.
- The previously used modules candidate
  `/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules` exists;
  its package metadata reports TypeScript 6.0.3, React 19.3.1 and Next 16.4.2.
- Prior full `full-tpvcnnp4/results.json` explicitly supplied profile source
  `/Users/zsolt.gajdacs/Library/Application Support/CodeCallNavigator`; that path
  and `catalog.sqlite3` exist. No private catalog contents or active-graph
  readiness were inspected. Future performance tests must use the runner's copy.
- The exact historical private Recurring Billing path was not recovered from the
  prior configuration inspected. The parent subsequently supplied the current
  fixture candidate `/Users/zsolt.gajdacs/Projects/Recurring-billing`; its `pom.xml`
  exists. This is not proof of repository access or successful compilation.
  Supply the required environment explicitly and let the category own an isolated
  fixture when building could modify the checkout.
- `.sonar/config.json` is absent. CodeScene remains mandatory; no new quality
  verdict is claimed by this read-only audit.

Full preflight also requires Apple Silicon macOS/Metal and process inspection,
Node 24+, JDK 25, .NET SDK 11, Maven, Graphviz and signing tools. Run outside the
sandbox. No native/display/package/performance readiness is established here.

Documentation discrepancy flagged to the parent: `docs/testing-strategy.md` and
`tools/verification/suites.py` disable Cargo automatic cache cleaning with
`CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never`, while `docs/reference.md` still says
age-based downloaded-cache management remains enabled. No out-of-scope doc edit
was made.

## Unit 3 red checkpoint remains separate

`target/verification/task22-unit3-red/04-unit-test-rust.log` records four expected
expanded-contract failures: structural owner address validation, visible call
origin serialization, full-address cardinality/distinctness, and explicit visible
stop markers. This is tests-first evidence, not a current implementation green.
The category cleanup report says passed, reclaimed 534,543,924 bytes, retained
zero, with no errors. Its timing report separates compilation 9.7s, setup 1.744s,
test 11.369s and cleanup 1.049s. No unrelated failure or retained-output cleanup
claim is inferred from that checkpoint.

## Independent final Unit 3 source/result audit

The producer's frozen result-contract implementation was independently reviewed
without executing the category again. Exact evidence:

- `target/task22-unit3-contracts/freeze-sha256.json`: all ten file hashes matched
  on two audit reads. Manifest SHA-256:
  `4941b72136687e42f366038d7e71f0c24d699735f0c8dbd45e6aa7f95d2a8acf`.
- `target/verification/task22-unit3-final/results.json`: all five category stages
  passed, with exit code zero.
- Actual libtest summaries and test-name lines independently total **142 distinct
  passed / 0 failed / 0 ignored**, matching 142 selected and zero not-run entries.
  The **17 expanded-contract cases are included in 142**, not additional passes.
- `unit/category-cleanup.json` reports passed, 537,849,438 bytes reclaimed,
  zero retained, no errors and no unmeasured paths. All paths recorded as removed
  were independently confirmed absent. No test process or build was started by
  this audit.
- Timings remain separated: prerequisites 0.891s, preparation 0.950s, compilation
  9.808s, tests 10.175s, cleanup 1.059s. These are category-stage observations,
  not a cold/warm benchmark or an isolated 17-test timing claim.

The tests exercise production public contracts with explicit in-memory inputs:

| Observable contract | Assertions inspected |
| --- | --- |
| Compatibility and serialization | A real ordinary `flow::build` chart retains exact golden chart bytes inside a root-only result; full result serialization/deserialization is stable. |
| Owned source interning | Exact BOM/Unicode/CRLF bytes and digest preserved; same owner/content deduplicates, distinct owner stays distinct, conflicting content under one owner rejects. |
| Stable structural sites | Equal spans across branches, nested expressions, deferred regions and catch/finally/cleanup blocks yield distinct typed owner paths and slots; static enumeration invents no execution copies. |
| Cleanup/repeated occurrence identity | Full original scope, pending route, outer-to-inner context, target certainty/provenance and repeated choices remain distinguishable through hashing and serialization. These tests prove identity semantics, not cleanup execution or assembly. |
| Visible source/origin integrity | Missing or duplicate full call origins, invalid IDs/links/regions, bad source digest/owner/index, reversed ranges and UTF-8 splits reject at `ExpandedFlow::validate`. |
| Stop/boundary integrity | Stopped work needs a visible nonblank marker and explanation; cannot claim assembled entry/return/throw boundaries or unrelated owners. |
| Result-local child correspondence | Both stopped and expanded children must match an exact displayed parent site and full target. Mutating certainty, provenance, relationship kind, call slot or explicit call boundary rejects; a valid setup is validated before each negative family. |

These are core **unit** contracts, not mocked compiler services. Mutating a valid
serialized result and observing the actual validator's rejection exercises the
external-data boundary directly. A shared fixture constructs data and calls real
ordinary chart building; it does not reproduce validation decisions. No unique
missing behavior was found within this unit's result-only scope, so no additional
tests or source changes were proposed.

The stronger child-correspondence red is separately retained at
`target/verification/task22-unit3-correspondence-red/04-unit-test-rust.log`
(15 contract passes / 2 failures), before the final green. Both final CodeScene
raw reports (`quality-final-old.json`, `quality-final-modern.json`) pass their
safeguards: nine scores are 10 and the declaration-only `expanded.rs` is null,
not an invented numeric score. Earlier failed quality reports remain preserved.

The engine-unit evidence still contains both pure TypeScript policy identities
passing. The original fabricated-worker source backup remains intact at
`/private/tmp/codecallnavigator-task22.7/crates/codecallnavigator-engine/src/analyzers/typescript_tests.rs`,
SHA-256 `df6b95c846b89123ee828e8a145b82758ec84ad09bdbe861c19e7e284d081e76`.
The private backup's Unit 2A/2B/2C manifests (22/19/16 files) still match. Its
earlier Unit 1 `flow.rs`, `model.rs` and `parts.rs` hashes were intentionally
superseded by partial Unit 3; this audit does not claim the whole old Unit 1 tree
is unchanged.

Scope remains limited: no expansion API, callee assembly, immutable-input IR
membership/eligibility validation, new consumer execution, native acceptance or
full verification is established by these result-contract units. Later assembly
must enforce input-backed eligibility and source ownership independently.

## Unit 4B single-call expansion audit and root-only seam

The original frozen Unit 4B was independently audited from source and
`target/verification/task22-unit4b-final`, without repeating its category merely
to obtain another count. Its actual log contained **155 unique passes, zero
failures/ignored**, matching 155 selected and zero not-run registrations. The 12
expansion cases were included, not added again. All five stages passed.
Preparation/prerequisites totaled 1.844s, compilation 9.522s, test execution
13.646s and cleanup 1.153s. Cleanup reclaimed 554,738,835 bytes, retained zero,
reported no errors/unmeasured paths, and all removed paths were absent.

All 18 source hashes matched the producer freeze during that audit. Its manifest
SHA-256 was `b54e4ca37c7535c2ccb0fe879909b87e2d11617e861a6fd4dbb40ac05f23e1b1`.
The three legacy invocation-fact IR/chart/artifact goldens still match Unit 1.
The eight-chart ordinary builder golden remains 17,644 bytes, SHA-256
`32fcd5279b59b32f42e48dc80cf5491af7c23c29effbada5c4a5638cd1d4a6e6`.

Observable topology assertions inspected:

- Child entry/body is reachable; removing the child entry prevents reaching the
  caller continuation, and child return boundaries resume the caller's original
  return or decision. The resumed decision has no duplicate call metadata.
- Child throw and unknown leaf failure reach child cleanup before caller cleanup,
  with an outer catch path. Pre-body call failure independently reaches the caller
  while bypassing the child. A throwing finally suppresses child return and the
  caller's otherwise normal continuation.
- Cleanup-copy selection identifies the original resource/finally/unpaired scope
  and semantic pending route, and expands exactly one matching copy.
- Missing/unknown/deferred/reference eligibility produces a visible stop, retains
  the caller continuation and does not retain rejected child source snapshots.
  Virtual/candidate selections preserve full uncertain target facts.
- Coincident spans and multiple calls do not manufacture isolation; stale target
  evidence or nonexistent cleanup context errors, and cancellation rejects.

`reaches` observes the returned graph using a visited-node set. `labels_from`
enumerates simple paths through these small explicit fixtures with a per-path
visited set, preventing cyclic revisits. It can be combinatorial on arbitrary
graphs and is not presented as a large-graph algorithm or runtime trace oracle.
Neither helper simulates lifetimes, eligibility, values, exception unwinding or
compiler decisions: those behaviors are asserted on actual production chart
topology, boundaries, source ownership and call metadata.

Both review defects are preserved in
`target/verification/task22-unit4b-review-red/04-unit-test-rust.log`: 10 expansion
passes and two failures. The final green proves a reference-only leaf does not
invent a callee throw boundary, while empty/mixed/unknown targets remain
conservative and pre-body failure remains possible. It also proves valid legacy
Action/implicit cleanup selection produces an ineligible marker rather than a
stale-selection error. Both final raw CodeScene reports pass; 17 scored files are
10 and the declaration barrel is null. Earlier failed/intermediate results remain
preserved by the producer.

### Unique missing seam found and covered

All 12 original expansion cases selected a root call. `ExpansionRequest::new`
also allows no selection, while the older root-only contract test wraps ordinary
`flow::build` directly. It therefore did not exercise the new expand → record →
remap → validate seam. This was a concrete coverage gap, not a demonstrated
production bug or a reason to duplicate existing tests.

The parent authorized and registered one new **core unit** case, with no external
requirements:

`codecallnavigator-core/test:flow_expansion::root_only::unselected_expansion_preserves_ordinary_cleanup_chart_and_owned_call_origins`

The test calls actual `expand` with no selection and only an immutable root input.
Its input includes acquisition/cleanup/finally, return/fallthrough branches and
multiple calls whose callee bodies are unavailable. It requires byte-identical
ordinary chart output, exactly one root occurrence and source, exact source bytes
and ownership, root-owned origins and complete call-origin cardinality. This
proves no eager callee lookup/assembly is required without an explicit selection.
It uses explicit input data and public production APIs, not copied builder rules
or a fabricated successful compiler. Only
`tests/flow_expansion/root_only.rs` and its module declaration in
`tests/flow_expansion.rs` were changed; both received CodeScene MCP scores of 10.

Because this added unique coverage, one category execution was justified:

```sh
./verify-core.sh --suite unit --output target/verification/task22-unit4b-independent-root-only
```

This ran outside the sandbox and passed all five stages: **156 distinct actual
passes, zero failures/ignored, 156 selected, zero not run**. Thirteen expansion
cases are included. Prerequisites took 0.894s, preparation 0.945s, compilation
10.740s, tests 11.942s and cleanup 1.152s. Cleanup reclaimed 555,263,299 bytes,
retained zero, had no errors/unmeasured paths; removed paths were confirmed absent.
No full/native/package or compiler run was added.

The original 18-file manifest differed only at the authorized test barrel before
and after this category; all production files remained unchanged. New source
identities after the run:

- `tests/flow_expansion.rs`:
  `87bfd21ce257fff575dcd223bb86b02af3eeb11c12404212b86802f3b71af7a8`
- `tests/flow_expansion/root_only.rs`:
  `ec089d3e07fcd1624afbded24617da04ece0e75e4fbae0fd0e3f8d53184a127e`

The category report and logs retain these actual test results and cleanup. Parent
quality/freeze integration follows the test-only addition. Scope remains one
selected root call or no selection: no recursive/multi-selection expansion,
configurable budgets, query/UI journey or native acceptance is claimed.

## Unit 5A nested-selection independent audit

The frozen nested-selection source/tests and existing final category were audited
without rerunning the same category. All **23** hashes in
`target/task22-unit5a-nested/freeze-sha256.json` matched on the audit reads;
manifest SHA-256:
`03e5c597c56353e4068de9815f19c4a733ea2816864de8b71db71ef08ce316c8`.

`target/verification/task22-unit5a-final` contains **167 distinct executed names
and passes, zero failures/ignored**, matching 167 selected and zero not-run entries.
The **22 expansion** and **19 result-contract** cases are included in that total.
All five stages passed. Prerequisites took 0.835s, preparation 0.917s, compilation
9.604s, tests 10.904s and cleanup 1.164s. Cleanup reclaimed 562,326,288 bytes,
retained zero, reported no errors/unmeasured paths, and removed paths were
independently confirmed absent. There is no additional tester execution or
isolated timing/performance claim.

Reviewed observable coverage:

- Nested selection yields the exact two-choice occurrence path, three separately
  owned sources, leaf reachability and return to the child's continuation.
  Nested cleanup also retains the semantic finally/return replay identity.
- Reversing both request order and immutable-input order yields byte-identical
  results. Diamond visits to a common callee and repeated sibling invocations
  remain distinct occurrences while sharing source snapshots. Possible target
  certainty remains possible. Direct/mutual cycles stop only on the active path.
- Stopped unknown or missing ancestors produce typed `blocked_selections` tied to
  a real stopped occurrence, with explanatory count, rather than synthetic child
  bodies/sources. Result validation rejects ghost call-bearing steps and rendered
  children under stopped parents.
- A nested stage ending in unresolved break rolls back child/leaf occurrences,
  request statuses and borrowed pending sources; no leaf body survives, only the
  caller source and an explanatory blocked request remain.
- Duplicate/orphan/stale selections, invalid unused input owners, duplicate symbol
  inputs, conflicting physical source bytes, malformed blocked choices, excessive
  count/depth and streamed encoded-identity size reject through production APIs.
- Two selected unproven slots share one original marker in original call-slot
  order with byte-identical ordinary chart output. Stopped conditions preserve
  the original single decision and exact ordinary literal-`false` routing.

These core unit tests use explicit immutable IR/source data and actual `expand`
and `ExpandedFlow::validate`; they do not fabricate successful compiler results
or duplicate eligibility/rollback decisions. The graph observation helper bounds
described above still apply. Source inspection additionally confirms pending
source content is borrowed (`&str`), original indexes are shared with `Rc`, and
owned source text is materialized only for a successful final result. The rollback
checkpoint contains small statuses and vector lengths, not cloned source bodies.
This is an ownership/code-path audit, not an allocation benchmark.

No unique missing boundary behavior was identified within 5A's scope. Runtime
reds are retained and correctly distinguished: invalid unused owner (19 expansion
passes / 1 failure), malformed request below unavailable ancestor (19 / 1), ghost
stopped body (18 contract passes / 1 failure), and shared multislot/condition
markers (20 expansion passes / 2 failures). The final green includes all these
identities. Both final raw CodeScene reports pass: 22 numeric scores are 10 and
the declaration barrel is null; safeguards pass. Earlier quality results remain
preserved. Legacy IR/chart/artifact and the exact 17,644-byte builder golden still
match their earlier hashes.

### Failed cleanup and later recovery remain separate evidence

The first `task22-unit5a-api-red` failed honestly at the missing-API compilation
step. Its test stage was **not run**, and its cleanup was also **failed** because
native lsof returned inconclusive nonempty PID/file data with exit 1. It retained
**498,850,158 bytes**. This historical run is not relabeled passed by later work.
Original result/log/cleanup/workspace records are preserved under
`target/task22-progress/cleanup-unit5a/original/`.

The separately recorded root inspection in `inspection.json` shows the exact
previously reported PIDs absent (`ps` exit 1 with empty output) and native lsof
with empty stdout/stderr. The later owned `recovery-cleanup.json` reports passed,
reclaimed exactly 498,850,158 bytes and retained zero, with no errors/unmeasured
paths. Its recorded removed build path is absent. The final category's successful
cleanup is a third, distinct observation. No cleanup rule was weakened or old
failed report silently replaced by this audit.

Remaining limits: no configurable expansion budgets or whole-attempt retry,
CLI/query/UI integration, compiler journey, package/native or full verification
is established by this nested core unit checkpoint. Only this audit document was
edited; no source/inventory write or build/test execution was performed.

## Unit 5B complete-assembly budget audit

Independent source/result inspection, without a redundant category rerun, confirms
all **14** frozen hashes in `target/task22-unit5b-budgets/freeze-sha256.json`.
Manifest SHA-256:
`abdcfd78a85cf1e8f6b6eb482bdc1788a41c2dd807be6bd81f66f502517443af`.

The actual final log under `target/verification/task22-unit5b-final` contains
**174 unique names/passes, zero failures/ignored**, matching 174 selected and zero
not-run registrations. All five stages passed. **29 expansion cases include the
seven budget cases**; these are not counted again. Prerequisites took 0.900s,
preparation 0.958s, compilation 9.190s, tests 11.729s and cleanup 1.089s. Cleanup
reclaimed 563,382,545 bytes, retained zero, had no errors/unmeasured paths, and
removed paths were independently confirmed absent. The producer records no
compilation warnings; this audit does not claim a new compilation run.

The tests assert actual public `expand` results, rather than reenacting budget
accounting:

| Boundary/risk | Observable assertion |
| --- | --- |
| Zero depth or reached-request allowance | Correct typed `Limit` reason, no child source, exact collapsed ordinary chart bytes. |
| Inclusive graph/source admission | Exactly measured complete graph/source allowances reproduce the full result bytes; steps or links independently one below the complete size demote the child and preserve the caller chart. |
| Work after an accepted graft | A later caller suffix or duplicated finally cleanup exceeding the allowance removes the earlier child graft, returns a typed Steps stop and preserves exact collapsed caller bytes/source count. |
| Nonmonotonic graph size | Full always-throwing B removes the suffix; demoting B restores the larger suffix, requiring another demotion of A. Both typed stops and exact collapsed chart are required, and reversed input/request order gives identical complete result bytes. No maximal expanded-subset claim is made. |
| Unique owner and oversized borrowed source | Same-owner root/child charge one source; a child longer than the old 16 MiB hard limit produces SourceBytes stop and blocked descendant with only the root source. The same oversized mandatory root returns an error. |
| Rejected nested work still costs requests | Child and leaf evaluations occur before rejection; later sibling is a Requests stop, with rolled-back leaf source/body and blocked metadata. Rejection does not refund already reached evaluation work. |
| Failure distinctions | Invalid ceilings/positive minima and exhausted mandatory-root allowances return errors. Entry cancellation remains an error containing cancellation context, not an apparently successful limit stop. |

Source audit confirms private `BudgetExceeded` is the only error type accepted by
retry downcasting. Malformed data and cancellation propagate normally. One
all-stopped assembly proves mandatory root fit; at most selection-count-plus-one
subsequent speculative attempts can add fresh demotions. The demotion set grows
monotonically; graph size is explicitly allowed to grow after demotion. A failure
without a new eligible candidate propagates instead of retrying forever. The
same assembler handles root proof and later attempts, avoiding a copied size
estimator or a second completion algorithm.

Each attempt resets its reached-work counter; rollback deliberately does not
restore that counter inside an attempt. Blocked descendants remain request
metadata and do not activate their missing/unbuilt bodies. `Prepared` shares
immutable indexes through synchronous `Rc` and scoped `RefCell` borrows; it does
not retain a mutable borrow across recursive assembly. SourcePool admission
checks the prospective unique byte count before adding a borrowed `&str` entry.
Failed attempts drop borrowed pools and private graphs. Only complete successful
materialization constructs owned source text.

The source-limit test proves absent leaked source entries and preserved caller
output. The stronger no-copy conclusion is limited to the inspected ownership
path, not an allocation/RSS measurement; the test's large input string itself is
allocated as explicit input. No synthetic successful compiler or mock business
service is involved. No unique missing required-scope case was identified.

The first API red is honestly a missing `ExpansionLimits`/`with_limits` compile
failure, not a manufactured runtime failure: the test stage was not run and owned
cleanup passed, reclaiming 506,450,506 bytes with zero retained. Both current raw
CodeScene reports pass, including safeguards: 13 numeric scores are 10 and the
declaration barrel is null. The exact 17,644-byte ordinary builder golden remains
unchanged. Historical 5A cleanup failure/recovery described above is unaffected.

Only these evidence notes were changed. No source/inventory edits, builds/tests,
compiler/CLI consumer, native GUI, package or full verification were performed by
this audit. The currently locked native session remains a separate prerequisite;
these core results do not establish native acceptance.

## Final negative eligibility matrix and focused SDK checkpoint

The parent identified one acceptance-criterion gap in the existing negative
matrix: explicit HTTP/navigation relations and deferred activation/callee facts
were not supplied, although production guards were already present. The same
test identity now supplies those four variants. Each requires an Ineligible stop,
no child body/source, exact full selected choice and preserved target metadata on
the original marker. Deferred activation uses valid partial facts
`{"timing":"deferred"}`, without a false isolation proof. No production change
or artificial runtime-red claim was made.

The first `task22-final-negative-contracts` core category passed 174 cases and
cleanup. Modern CodeScene scored the file 10, but the old CLI's global review
still found the enlarged assertion function too long. That failed verdict remains
preserved. With parent authorization, a cohesive `assert_ineligible_choice`
expectation helper extracted only assertions, without reproducing eligibility
decisions. Both old and modern scoped raw reviews now score 10, stored at
`target/task22-progress/negative-contracts-quality-{old,modern}.json`.

The source change justified the single subsequent core category
`target/verification/task22-final-negative-contracts-refactored`: **174 passed,
zero failed/ignored**. All five stages passed; prerequisites 0.900s, preparation
0.949s, compilation 10.336s, tests 11.670s, cleanup 1.107s. Owned cleanup reclaimed
563,500,018 bytes, retained zero. Final test source SHA-256 is
`c44183ba87175ee448e87c29acf79bcd03a32a9f69fe5fb67ed11cda34e134a7`.
The test identity did not change; scoped rustfmt/diff checks passed.

The first focused SDK attempt is retained at
`target/verification/task22-unit6-focused`. It used production inventory selection,
Runner/Step/exclusive locking, SuiteContext release environments, prerequisite
checks, exact Rust registration/execution and owned category cleanup. It selected
only one Java integration identity and two C#/TypeScript acceptance identities,
with all other registrations/gates explicitly not run. It is not whole-category
verification and never used fabricated compiler results.

- Java passed. Its setup/compilation/test/cleanup stages took
  0.091s/22.255s/1.993s/1.108s, plus prerequisites 0.151s. Cleanup reclaimed
  175,370,532 bytes and retained zero.
- The shared acceptance build compiled both targets once (24.688s), but the
  TypeScript test failed before expansion: the helper selected by CallSite label
  `alias()` instead of actual callee-expression label `alias`, finding zero sites.
  The production runner stopped at that artifact failure, so C# was **not run**.
- Acceptance cleanup passed and removed its owned build/temp paths. Its reported
  178,727,250 retained bytes are the two existing previous-app rollback packages
  deliberately preserved by cleanup policy, not leaked category workspaces.
- Source hashes were unchanged across the failed attempt. Full logs, exact
  selection/source snapshots, timings and `independent-observation.json` remain
  preserved; the failed attempt is not relabeled passed.

Read-only inspection of retained actual compiler output resolves the fixture
expectation: exact call span 104..111 is `alias()`, its label is `alias`, and its
full target is the implemented `leaf.ts|74|function`, with signature
`(value?: number): number`, `body_unavailable=false`, and one matching owned
synchronous FunctionFlowIR (span 74..118). The current fixture source hash equals
the saved source hash. This demonstrates an expectation/selector error rather
than an observed overload-to-body binding defect. Full observation is in
`target/task22-progress/typescript-callsite104-target-observation.json`.

The producer owns the authorized shared-helper correction: select an exact
compiler-owned span whose **cached** source slice equals known fixture call text,
then independently assert the real CallSite label and precise callee metadata.
Uniqueness/facts/source/connection assertions are retained. Because that shared
helper and Java Pair changed, the parent requires all three exact workflows on
one refreshed four-file freeze at a fresh `task22-unit6-focused-r2` output. That
repeat is justified by changed shared test source; it is not a parser/counting
rerun or a whole engine SDK suite. Its result is pending at this audit checkpoint.

### Final focused SDK retry

`target/verification/task22-unit6-focused-r2` passed **all three exact real SDK
identities** and all ten stages. Java integration passed one test; the separately
owned acceptance workspace passed TypeScript and C# tests built together once.
No ignored/skipped case was silently counted as executed. The inventories record
247 other integration test registrations/one gate and 80 other acceptance
registrations/five gates as not run. This remains focused-subset evidence.

Prerequisites/setup/compilation/test/cleanup respectively took
0.262/0.083/24.418/1.973/1.047s for integration and
0.188/0.077/24.861/14.450/1.212s for acceptance. Both cleanups passed and removed
their owned build/temp paths. Integration reclaimed 175,373,894 bytes and retained
zero. Acceptance reclaimed 234,048,959 bytes; its 178,727,250 retained bytes are
the two existing rollback app packages, deliberately preserved by policy.

All 45 reviewed source hashes and the exact inventory hash matched before/after
the retry. The four compiler-test source manifest has SHA-256
`609da4e53ec9a11d5dbb6dbaef29f0e15d5e61ee3cd145e3c4e890a8fa559271`.
Both final raw CodeScene reports pass four scores of 10 and their safeguards.
The completed archive is `evidence/flowcharts/expansion/unit6/`, with exact
selection, source checkpoints, commands, timing/cleanup records, both expectation
corrections, original failure and final negative-core 174 evidence. All 84
reports/logs in its tar archive were independently hash-verified after writing;
no retained binaries or packages were archived.

These are the sources at the focused SDK checkpoint. Root subsequently reported
the mandatory full run stopped on a core `calls.rs` collapsible-if Clippy finding;
cargo-check passed and later runtime stages were not run. Root owns that small
correction and full retry. The focused archive does not assert later entire-task
source immutability or a full/native/package pass.
