# C# cleanup and conservative control flow

TASK-22.3 uses existing shared version-1 IR for a bounded synchronous subset.
The [support matrix](../../../../docs/function-flowcharts.md#c-support) separates
structured lowering, conservative exception paths and opaque constructs. Native
identities, relationships and core production completion semantics stay intact.
Flow collection records compiler declaration owners to withhold ambiguous bodies.

## Focused evidence

[cli-acceptance.json](cli-acceptance.json) records 32 real .NET cases and 202
observed events: 23 supported observable path projections and nine explicitly
partial probes. All 40 ordinary forbidden traces and both filtered-region
missing-finally traces are rejected. Observed operation outcomes constrain
exception edges; observed MoveNext/continue booleans constrain their decisions.
The matcher preserves an observed pending exception through normal cleanup.

Genuine C# lock syntax is checked with gate-evaluation events, Monitor.IsEntered
inside the body, another thread's release probe, and source-owned structural
checks for capture, Enter dominance and guarded Exit. Monitor operations are not
claimed as directly instrumented runtime calls. Null/nullable guards are verified
as conditional structure, without a general runtime-value interpreter. Partial
async/filter/goto/guard/budget probes do not claim complete execution simulation.

The frozen native suite has 355 tests: 326 pass and the same 29 recorded historical
failures remain. Three core completion proofs pass. Four ignored production CLI
C# tests pass, including successful publication of a valid cross-file partial
method whose body cannot safely bind the retained native declaration location.
Raw graph/source/collision/budget/nameof parity is recorded in
[raw-cli-acceptance.json](raw-cli-acceptance.json).

## Tests-first and review checkpoints

- Six initial native cleanup tests failed against the preceding exporter.
- Three async-boundary reproductions failed before the suspension fix.
- [red-cli.json](red-cli.json) shows the actual using trace rejected by the old
  opaque chart; [first-green-cli.json](first-green-cli.json) shows it matched by
  structured cleanup and rejects a trace that omits disposal.
- [provenance-red.json](provenance-red.json) preserves real raw CLI outputs that
  concatenated bodies under aliased local/generic identities. Compiler-owner
  proof now withholds those flows. Valid-source ownership regressions assert
  zero compiler body errors and empty declaration diagnostics; the intentionally
  invalid accessor case separately preserves its compiler diagnostic.
- [catch-matcher-red-cli.json](catch-matcher-red-cli.json) records a test-matcher
  issue with normal disposal preserving an earlier exception into a catch.
- Typed-fixture conversion and constructor-negative checkpoints are retained in
  [typed-filter-matcher-red.json](typed-filter-matcher-red.json) and
  [typed-constructor-negative-red.json](typed-constructor-negative-red.json).
  Explicit typed identities and preconstructed finally markers remove fixture
  ambiguity without broadening the matcher or changing producer semantics.

The initial safeguard caught loose telemetry arguments and duplicated foreach
fixtures. Typed event/fault/resource identities and shared fault-parameterized
source bodies resolve those findings while preserving real break/continue syntax.
All project-owned source files are 10.0. The first full run exposed a missing
numeric score for a top-level-only fixture; moving its real exit-report hook into
that executable as a local function preserves the lifetime test and makes the
file scored. The exact corrected quality gate and packaged 32-case acceptance
pass. Existing upstream findings remain stable or improve. [code-health.json](code-health.json) records source hashes and findings.

## Repeat the acceptance

```sh
python3.13 tools/csharp_cleanup_acceptance.py --engine target/release/codecallnavigator-engine --output /private/tmp/csharp-cleanup.json
```

The helper copies its checked-in fixture to a disposable root, builds with the
ordinary .NET 11 SDK, and uses production CLI generation-bound queries. It needs
no frozen historical engine for routine execution. Full verification runs it
explicitly after building the release engine and retains a per-run report.

[verification.json](verification.json) records the full-verification status.
Full verification `full-bkd4h8bc` passed all 34 stages outside the macOS sandbox
with an isolated profiling snapshot. The integrated [final report](full-cli-acceptance.json)
records all 32 cases against the final patch identity. The manual final report predates a test-only validity-audit patch change;
producer DLL hashes stayed identical, and the integrated full gate records the
final patch identity.
