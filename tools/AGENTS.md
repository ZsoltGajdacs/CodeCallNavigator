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
./verify-changes.sh --phase development --run
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

During implementation and while fixing failures, use
`./verify-changes.sh --phase development --run` for test execution. Keep
behavioral combinations in tests of production logic with explicit in-memory
inputs. Isolate decisions from external effects at natural boundaries and
retain real boundary tests. Follow the root guidance on avoiding mocks,
fabricated results and test-only behavior.

After the last edit, use `./verify-changes.sh --run` to execute the complete
computed final plan. Include the entire change with `--base REF` when needed.
Fix failures and rerun the final plan. Run automatically selected full
verification outside the macOS sandbox and report unavailable prerequisites
as not run.
