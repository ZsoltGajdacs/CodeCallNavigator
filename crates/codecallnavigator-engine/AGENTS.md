# Analysis engine practices

Read the workspace practices and mandatory policies in
[AGENTS.md](../../AGENTS.md). This guide applies to `codecallnavigator-engine`.

## Responsibilities

The engine owns standalone command handling and compiler-backed analysis.
`entrypoints/` owns CLI/protocol policy; `analyzers/` owns orchestration, process
execution and language adapters. Depend on core contracts without depending on
the desktop, Rune or WGPU. Keep the standalone build independent of UI resources.

- Keep entry points thin. Share the production analysis pipeline between CLI and
  desktop protocol requests rather than duplicating analyzer behavior.
- Keep build-tool discovery/preparation separate from symbol normalization,
  graph analysis and framework inference. Keep language-specific rules with the
  relevant adapter; the coordinator owns lifecycle and composition.
- Normalize into core types at the adapter boundary. Add languages through the
  explicit dispatch and [extension contract](../../docs/standalone-engine.md),
  rather than introducing a speculative plugin framework.

## Analysis and process invariants

- Use compiler bindings and verified source/bytecode attribution for identity
  and ownership. Do not substitute name matching for resolved symbols.
- Preserve native bindings, relationship kinds, certainty, source spans and
  provenance. Keep possible invocations, deferred callbacks and references
  distinct; unresolved ambiguity should remain visible through diagnostics.
- Use the same resolver for graph edges and flow targets. Framework registration
  alone must not create an invocation or establish runtime execution.
- Preserve snapshots and fingerprints across preparation and analysis. Reject
  incomplete output, concurrent input changes and incompatible adapter resources
  rather than publishing a partial graph as a successful refresh.
- Propagate cancellation and client disconnects to owned subprocesses. One-shot
  commands receive closed stdin; persistent worker protocol pipes remain explicit.
  Keep machine-readable stdout free of logging and report failures contextually.
- Run heavy operations in parallel where work can safely run concurrently. Use
  the tool's native scheduling and CPU detection where they support the target
  of half the available logical CPUs, rounded down to a minimum of one worker.
- Preserve the versioned process protocol, artifact digest verification and
  atomic publication contract. Failed or cancelled work retains prior results.
- Update resource capability/version checks, bootstrap locks and packaging
  together when adapter contracts change. Executables and adapters must agree.

## Testing strategy and verification

Follow [the shared strategy](<../../.backlog/docs/testing/doc-12 - Testing-strategy.md>). Put argument rules,
normalization, inference, ownership and scheduling policy combinations in unit
tests of real isolated production logic with explicit in-memory inputs. Extract
cohesive policy from build orchestration when it improves production design.

Use integration tests with real minimal compiler/build fixtures for adapter
semantics: symbol identity, ambiguity, unsupported input, source attribution,
changed inputs and failed publication. Real compiler tests remain integration
when they bypass the public engine command. End-to-end tests exercise the actual
public executable with real dependencies, including analysis, queries, recovery
and packaging. Retained artifact query fixtures need no recompilation to prove
query behavior; retain separate compiler-to-artifact boundary checks.

```sh
./verify-changes.sh
./verify-changes.sh --phase development --run
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Run from the workspace root. Inspect the report-only command for selected JDK/.NET/Node,
resource, SDK and build-tool prerequisites; report unavailable tools as not run.
Category scripts clean owned Rust outputs and temporary fixtures even on failures,
retaining logs/evidence and downloaded caches. Use `--keep-artifacts` only for
explicit diagnostics. The selector automatically requires full verification for resource/bootstrap/packaging changes.

Do not mock successful analyzer/build business behavior. Document narrow process
fault probes for unreliable failures (blocked pipes, stalled children, malformed
frames) as integration coverage, never successful real-dependency acceptance.
Explain the unique risk of each expensive test and why a cheaper layer is
insufficient. Removal requires assertion mappings and demonstrated defect detection.
Preserve uncertainty, exact source ownership and byte-identical prior artifacts on
failure/cancellation. Roughly one minute of warm unit execution is aspirational;
never weaken verification or timeouts to meet it. Follow root CodeScene and
scoped-formatting rules.

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
