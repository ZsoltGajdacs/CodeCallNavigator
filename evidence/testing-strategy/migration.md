# Testing strategy migration evidence

The migration remains on the existing branch and preserves all existing test
function definitions. No test deletion or claim of equivalent replacement coverage
is recorded in this batch. `baseline.json` captures source/executable counts,
observed machine/tool versions, historical reports and current category observations.
Those reports have different selections/cache conditions and are not directly
comparable full-suite benchmarks.

## Review refinements

The source/helper audit in `classification-review.json` identified thirteen
registrations that exercise a real file worker or process/protocol orchestration
and fault probes. They move from unit to integration without changing their test
bodies. Inherited successful OS protocol simulations are explicitly identified;
they do not prove successful Homebrew installation, screenshot capture or native
window mutation. Real packaging and native journeys remain separate coverage.
No mocked analyzer or graph decision is introduced by this work.

`core-placement.json` maps a further 72 core-owned registrations (32 unit, 40
integration) into the core package, preserving 264 assertion macros. Twelve files
move with only namespace changes and required formatting; three core cases and
their graph fixture move out of the mixed entry-metadata target. The existing real
C# compiler case stays in the root package so its manifest-relative fixture paths
are unchanged. Together with the first eleven flow cases, this removes desktop
and Rune dependencies from the core component's test selection. The focused core
unit run passed 109 registrations; its compiler artifact records contain only core
test targets, with 9.597 seconds compilation and 8.087 seconds execution.

Serialization is now independent of category. Source inspection found concrete
shared resources in the private Java checkout, repository MediatR solution and
profiling machine/store. Their seven Rust registrations have explicit serial
groups. Other correctness groups use bounded four-worker parallelism; unit groups
use libtest defaults. Artifacts remain sequential, and Python remains serial
because its bootstrap/CI cases mutate interpreter globals. A category label alone
no longer serializes a Rust target.

The blocked-output acceptance probe originally assumed a pipe queue byte threshold
that does not establish backpressure on this Mac. Its replacement uses a real
bounded Unix socket, suspends draining only after a real initial CLI frame, fills
the physical transport and checks kernel write-readiness before real compiler
work. Filler bytes are never decoded as business frames. The shutdown deadline,
successful exit and descendant checks remain intact. The focused packaged-CLI run
and all fourteen scenarios in the final full verification passed.

Cleanup regressions exercise actual success, failure, timeout, SIGTERM during a
running child and between-stage recording, active held files, ownership rejection
and explicit retention. Already-cleaned reclamation now preserves the original
cleanup receipt instead of overwriting its reclaimed-byte count with zero.

## Coverage mapping

| Prior execution | New execution | Retained verification |
| --- | --- | --- |
| Ordinary Rust suite in debug, repeated by the full release command with `--include-ignored` | Each exact executable registration is assigned one category; ordinary tests execute there once | Same test bodies/assertions, runtime registration checking and real dependencies. A documented release requirement selects the proper profile rather than rerunning the whole suite. |
| Ignored tests bundled with ordinary tests and canvas benchmarks | Explicit integration, acceptance and performance categories with requirements | Compiler/build, CLI/MCP, native UI, storage and CPU/GPU checks remain selected by their actual boundary and required environment. |
| TypeScript `observations.test.js` two in-memory cases run with all compiler tests | Two exact names run in unit; remaining 110 Node cases run in integration | `different values observed during preparation are rejected immediately` still checks inconsistent repeated observations; `unchanged observations may be queried repeatedly and verified` still checks stable repeated reads. Same production `Observations` implementation and original assertions. Filesystem changes and compiler behavior remain real integration coverage. |
| Root-package `tests/flow_core.rs`, compiled with desktop dependencies | Core-package `crates/codecallnavigator-core/tests/flow_core.rs` | All eleven cases and 45 assertion macros are preserved. Ten remain unit cases; the real Graphviz layout case remains integration coverage with its Graphviz requirement. Only the crate namespace import changes. |
| Full verification's mixed release acceptance/benchmark timing | Separate compilation, setup, execution, cleanup and other stage costs | Functional acceptance and performance failures remain failures; exceeding a timing aspiration does not fail a correct run. |
| Cleanup only at composite end | Cleanup after every category, including standalone invocation and failure paths | Existing ownership/inactivity protections, process reaping, downloaded-cache/evidence retention and explicit diagnostic retention remain required. |

Source-case identity and executable registration are separate: Rust lib/bin targets
may register one source case in two executables; Python inherited test methods can
register in multiple concrete classes. Both executable registrations remain covered.
The earlier conversation's 1,071 source estimate omitted Rust integration tests in
the root `tests/` directory and cannot serve as the complete inventory baseline.

## Core flow placement batch

The flow target now belongs to the core crate that implements the exercised
operations. Running `cargo test --locked -p codecallnavigator-core --test flow_core`
therefore does not compile the desktop, Rune or the engine merely to test shared
flow rules. The target intentionally retains its real Graphviz case; moving files
does not turn that boundary test into a unit test. Category scripts still select
the ten unit cases and the Graphviz integration case separately.

[The placement proof](flow-core-placement.json) records every old-to-new executable
identity and SHA-256 hashes. Byte comparison confirmed the destination equals the
original file after the single `codecallnavigator::` → `codecallnavigator_core::`
namespace substitution. Every helper, test body and assertion is unchanged; no
test was removed, replaced or reclassified. Category counts are unchanged.

This is a dependency-placement improvement, not a measured timing claim. The
previous unit run exercised the old registrations; post-move focused execution
must establish the new target's compiler/runtime result. Timing and cleanup
outcomes belong to that actual verification report.

## Remaining core test placement batch

A further 72 registrations (32 unit, 40 integration) move from root-package
targets to the core crate: twelve complete test files and the three core cases
from `entry_metadata.rs`. Its shared `graph()` helper moves with those three
cases. The real C# entry-metadata analyzer case remains in the root package;
its body and manifest-relative fixture paths are unchanged. No core-owned Rust
test registration remains attached to the desktop package after this batch.

[The complete mapping and preservation proof](core-placement.json) records all
72 old/new executable identities, per-file or per-item hashes, and all 264 retained
assertion macros. Categories, requirements, purposes and test bodies are preserved.
Twelve whole files initially matched their original bytes after only replacing
`codecallnavigator::` with `codecallnavigator_core::`. Four then required rustfmt
wrapping; their final bytes were compared with deterministic rustfmt output of
those exact namespace-only intermediates. Both intermediate and final hashes are
recorded. This is not whitespace stripping, which could hide changed string data.

Core unit/integration scripts can now compile the owning core package without
pulling the desktop/Rune/engine merely because tests were placed in root targets.
No test is removed or reclassified by this placement batch. The focused core run
and final full/light category executions passed the new targets. Earlier full-run
reports retain their original selected identities, linked
to the new registrations through the preservation mapping.

## Private Java fixture build scope

Only these two existing engine integration registrations append
`-DskipUi` to their child process's `MAVEN_OPTS`:

- `codecallnavigator-engine/lib:codecallnavigator_engine::analyzers::java::compiled_acceptance::recurring_billing_successful_build_outputs_preserve_both_generated_copies`
- `codecallnavigator/test:java_duplicates::recurring_billing_retains_duplicate_generated_declarations`

The private fixture's inspected Maven configuration supports the `skipUi` property:
its `no-ui-build` profile activates on that property and its frontend Ant target
has an `unless="skipUi"` guard. The flag excludes unrelated frontend compilation
and Vitest work from these Java analyzer checks. Real Java compilation, generated
outputs, duplicate declaration checks and every existing assertion remain intact.
No private checkout files or global environment settings are changed. Other Maven
tests do not receive the flag.

The inventory's optional `environment_append` mapping defaults to empty and
validates environment names and string values before execution. The runner must
append the flags to inherited child values rather than replace existing Maven
options. Invalid mapping shapes, names, non-string values and embedded NUL values
are rejected by the existing metadata regression test.

Both original Java cases passed in the focused engine integration and final full
verification. The flag changes only those child environments. Observations across
different suite selections and cache conditions do not establish a controlled
full-suite speedup.

## Archived C# fixture temporary ownership

The raw-flow and ingestion gates previously recreated archived physical fixture
paths beneath `/private/tmp/task22-flow-baseline-source` and
`/private/tmp/task22-ingestion-baseline`. Their project/build outputs therefore
escaped the category's owned `TMPDIR`, process-use checks and cleanup accounting.
Future runs materialize identical source/project bytes beneath each gate's
existing `TemporaryDirectory` work instead; old fixture directories are neither
adopted nor deleted.

[The expected-data relocation proof](csharp-fixture-ownership.json) records the
only expected snapshot changes: 27 physical-path/identity strings across the
three ingestion native projections and one raw-export architecture source path.
Rebasing requires an exact old-root value or a complete old-root-plus-slash prefix;
similar names and embedded source text remain unchanged. Canonical projections
and historical artifacts remain exactly as archived. Actual compiler output is
never normalized, and complete projection/field comparisons, source-byte checks
and flow determinism assertions remain in place.

Five focused regressions exercise prefix boundaries and snapshot immutability,
real materialization of all three encodings, source SHA-256 preservation, owned
raw-export source paths, and refusal of symlinked or escaping source/project
filenames. They do not fabricate compiler success or establish passing compiler
acceptance. Both actual compiler gates subsequently passed, including their final
full-composite invocations.

## Observed execution

`target/verification/unit-f2lmlps5/results.json` records a completed unit run:
48.965 seconds compilation, 2.429 setup, 57.521 test execution and 2.393 cleanup.
`target/verification/unit-_sj7s_5c/results.json` records the smaller Python-only
observation at 9.850 seconds execution. These are observations of the selected
revision/tests at those runs, not promises for later inventory additions or a
certified cold/warm benchmark. Inspect each retained `selection.json` for exact
selection; do not infer passing-test counts from selected registrations.

Historical `full-l9fmsdlu` recorded 226.394 seconds ordinary Rust execution and
1774.885 seconds combined release acceptance/benchmarks. `full-k7cvs5hs` recorded
202.976 and 1762.168 seconds respectively. These figures identify prior cost centers,
not the speedup of the new complete full composite. Historical machine/cache state
was not recorded; current machine metadata in `baseline.json` is labelled as current.

## Final validation

The pure reporting regressions verify that failed-stage execution and cleanup costs
remain visible, selected/omitted cases and gates stay separate, source-case counts
differ from executable registrations, and interrupted/not-run stages never become
fabricated passing-test counts. `tests/test_verification.py` separately checks the
shared category plan, isolated build paths, retained package/install/agent journey
contracts and wrapper dry runs without evidence writes.

[The durable validation record](validation.json) records:

- `full-enbwvtxc`: all 112 stages passed, including real compiler/CLI/MCP,
  installed packages/signatures, native light/dark journeys, Metal and storage
  benchmarks, CodeScene and cleanup. Python integration ran 258 tests without skips.
- The full selections cover all 1,358 registrations and eleven standalone harnesses
  exactly once, with no missing or duplicate identities across categories.
- `light-9t8sexpr`: all 83 stages passed; 407 unit and 635 ordinary integration
  registrations selected, with compiler-backed/acceptance/performance omissions
  explicitly reported.
- Full unit execution took 17.795 seconds; compilation took 61.037, setup 0.958
  and cleanup 4.962 seconds. This is an observation, not a timing acceptance gate
  or a certified cold/warm benchmark.
- All four test-category cleanups passed and reclaimed an estimated 8,941,891,272
  logical bytes. Required packages/rollback evidence remain; no category build
  or external temporary workspace remains for that full run.
- CodeScene reviewed all 51 changed source files successfully. Tool versions are
  recorded in the full summary, baseline and `toolchains.json`. SonarMCP is inactive
  because this project has no `.sonar/config.json`.

The final native gate launches are proven beneath their category-owned scratch
directory, which is absent after cleanup. Failed/interrupted native fixtures are
token-bound and moved into retained gate evidence only after inactivity checks.
Real-file tests cover active handles, unsafe bindings, incomplete tokens and
interrupted record writes; failure reports include measured retained bytes and
unknown-size paths. A transient inconclusive native inspection in an earlier
attempt correctly preserved its build; a later independent idle proof allowed
guarded reclamation. That earlier full attempt remains recorded as failed.

Before removing any future expensive test,
record its identity, defect scenario, assertion-to-retained-test mapping, measured
cost and demonstrated failure detection. Keep unique boundary coverage even if slow.
