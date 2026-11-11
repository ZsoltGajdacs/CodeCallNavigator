# Java equality inference verification — 2026-11-11

TASK-35 disables java-callgraph2 type/value inference only for exact
`equals(java.lang.Object)` and `hashCode()` signatures. The analyzer retains
their declarations and bytecode call targets. Other methods, including overloads,
retain full inference and the existing strict completeness checks.

## Reproduction and regression checks

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

## Private reactor verification

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

## Build and quality checks

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
