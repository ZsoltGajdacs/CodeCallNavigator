# Bound saved-revision flow queries

TASK-22.5 adds the optional version-1 public CLI `flow_comparison` operation.
The [wire contract](../../../docs/cli-queries.md#saved-revision-function-flows)
describes catalog paging, selected before/after details and explicit manual
counterparts. These checks exercise real Git repositories and CLI subprocesses;
desktop comparison presentation belongs to TASK-22.6.

The unit reports preserve focused evidence from an isolated checkout:

- `first-unit-verification`: portable artifact flow ownership, cached source,
  ordinary Flow parity and Code Health.
- `second-unit-verification`: strict revision inspection, unknown schema,
  exact-byte provenance and unchanged legacy Compare bindings.
- `third-unit-verification`: tests-first contract/CLI failures followed by 163
  passing focused tests, all-target Clippy and both installed CodeScene versions.
  One existing SDK-dependent ordinary-query test was ignored in this focused run.

Catalogs use exact symbol identity. Incompatible endpoint membership is unknown;
it never establishes a removal or addition. Ready details use endpoint-local
source/targets. Global strict artifact validation remains in place; selected IR
conversion is lazy only after that validation. The implementation avoids
hydrating two graphs or constructing two GraphIndexes.

WORKTREE freshness retains existing read-only live-input observation. Tests
modify and delete a live file, observe stale freshness, and assert identical
cached-source detail results. Committed endpoints require no linked checkout.
Tests also verify moved refs, changed saved bytes, cancellation, no fetch or
checkout, unchanged canonical/cache files, complete-envelope byte limits and
legacy query/session compatibility.

The independent final audit adds five CLI cases: 168 focused tests passed, with
the same existing SDK-dependent test ignored only in the focused run. Its
`third-unit-verification/independent.json` distinguishes fixture corrections from
production defects.

Integrated root full verification passed all **62/62 stages** in
`target/verification/full-k7cvs5hs`: workspace compilation/Clippy, ordinary suites,
TypeScript tests, real compiler fixtures, all ignored acceptance tests in release
mode, isolated profiling/benchmarks, package/signature checks, installed CLI/MCP
workflows, CodeScene and cleanup. See `verification.json` and
`full-verification/results.json`; raw logs remain in the named local run folder.
No new native UI behavior is claimed for this CLI/core task. TASK-22.4's separate
native C# workflow evidence remains applicable to the unchanged desktop.
