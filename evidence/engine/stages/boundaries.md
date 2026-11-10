# Navigation and HTTP boundary verification

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
