# Tooling testing practices

Follow [root guidance](../AGENTS.md) and [the testing strategy](<../.backlog/docs/testing/doc-12 - Testing-strategy.md>).

Prefer unit tests of parsing, test selection, normalization and report evaluation
using explicit inputs. Use real temporary directories, Git repositories, archives,
SQLite and subprocesses for integration tests of filesystem, packaging and runner
behavior. A test of an acceptance-report evaluator does not establish acceptance
of the application. Keep installed-package and real host journeys distinct.

TypeScript analyzer tests belong to the engine component: pure observation/policy
logic is unit coverage; real compiler, module resolution and filesystem extraction
are integration coverage. Keep Java/compiler helpers tested through real minimal
source/build fixtures. Do not fabricate successful compiler or CLI business results.
Only documented integration fault probes may simulate otherwise unreliable process
or protocol failures. Name the risk and cheaper-layer limitation for expensive tests.

```sh
./verify-changes.sh
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Run from the workspace root and inspect the report-only command for prerequisites. Match tests
to behavior rather than file names. Preserve failure, timeout, interrupted,
cleanup-unavailable and not-run results; never turn missing prerequisites into pass.
Category cleanup must prove ownership and inactivity, reap processes before removing
builds/fixtures, preserve caches and evidence, report reclaimed/retained bytes, and
fail on uncertainty. Test success, failure and interruption cleanup with real
resources. Do not add mocks merely to test runner implementation calls.
Report build/setup/execution/cleanup costs separately. The roughly one-minute warm
unit aspiration is not an enforced threshold. Removal requires a coverage mapping
and demonstrated failure detection; keep unique boundary checks regardless of cost.

Agents must use `./verify-changes.sh` for verification selection and
`./verify-changes.sh --run` for all test and verification execution. Do not
substitute raw test commands or manually selected category/component scripts.
Execute the complete computed plan; full verification is selected automatically
by checked-in rules, with no manual scope override. Use `--base REF` to include
committed changes. Run automatically selected full verification outside the macOS
sandbox. Report unavailable prerequisites as not run, never as passing checks.
