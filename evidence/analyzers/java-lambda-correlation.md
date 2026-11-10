# TASK-32: Verified Java lambda identities

Implemented on `codex/standalone-graph-engine`, 2026-11-10. No upstream
dependency changes, merge or push are needed.

## Behavior and proof

- The `kind_v8` inventory snapshots source lambda facts before lowering. A
  processor-free, memory-only javac replay captures source offsets and exact
  LambdaMetafactory implementation handles.
- A shared BCEL executable-class model checks class version/ownership/hierarchy,
  field and method descriptors/flags, generic signatures, constants, symbolic
  instructions, exception handlers and expanded bootstrap arguments. It rejects
  duplicate members. Debug attributes and constant-pool positions are excluded
  from this proof. Both debug modes must agree when both match the actual output.
- Existing singleton source keys are unchanged. Complete shared implementations
  receive one tagged, sorted member collection; membership/body changes can
  change that identity. Indistinguishable separate peers retain deterministic
  occurrences without individual continuity claims after identical swaps/inserts.
- Identity assignment and source-flow binding are separate. Shared implementations
  never acquire an arbitrary member flow. Native nodes and full native edge
  records are unchanged by binding, including absent SourceFile/line metadata.
- Verified and rejected owners cannot use legacy line fallback. Unavailable
  compiler replay retains conservative legacy attribution and specific reasons.
  Local/anonymous owners retain compiler-native portable-identity warnings.
- The inventory version participates in cache keys and analyzer provenance.
  Runtime helpers require v8; packaging rejects incomplete/legacy adapter jars.

## Verification

| Check | Result |
| --- | --- |
| Regression-first real JVM and full analyzer checks | Old v7 lacked correlation records; same-line flow count was 1 instead of 3 |
| Workspace tests (`cargo test --workspace --locked`) | All unignored tests passed |
| Strict workspace all-target Clippy | Passed |
| Focused Java flows, lambda identities and correlation | 12 passed |
| Maven/Gradle, duplicate classes, HTTP, Lombok, Spring, packaged resources | 20 public acceptance tests passed |
| Public real-JVM Java engine-library checks | 9 passed |
| Python suite | 72 passed |
| Final adapter `installDist` | Passed; compiler-internal exports included in helper launchers |
| Engine-only packaging | Passed; installed and packaged Java adapter jars match |
| Final packaged engine process | Passed: one merged native lambda, one grouped identity, no arbitrary source flow |
| Independent reviewer | No actionable findings |
| CodeScene file reviews | All 22 supported modified/new source files scored 10.0 |
| CodeScene pre-commit safeguard | Passed: 22 of 22 eligible source files, no regressions |

The real-javac controls cover `-g`, `-g:none`, `-g:vars`, target release 17,
same-line and nested lambdas, captures, serializable lambdas, field/static/
constructor initializers, method references, merged nested/sibling implementations,
source/output immutability, stale source, and tampered bootstrap and method
descriptors. Rust boundary checks reject incomplete/duplicate/conflicting groups,
cross-owner/source-file evidence and contradictory known file locations.

End-to-end checks cover exact singleton spans/targets without debug metadata,
unchanged keys after unrelated insertion/reordering, body edits, group membership
edits, deterministic repeats, checkout relocation and export/hydrate/export.
Existing inventory cancellation, cleanup and atomic publication regressions pass.

## Limits and execution notes

- The private recurring-billing fixture was unavailable and explicitly skipped.
- No `.sonar/config.json` exists, so SonarMCP is inactive.
- CodeScene does not support `.kts`; its installation verification passed 5/5.
  The Gradle script was verified by the actual adapter build instead.
- Maven initially hit a sandboxed global cache lock. Offline execution with
  `-Daether.syncContext.named.factory=rwlock-local` resolved it. Gradle acceptance
  required access to existing tool caches outside the sandbox.
- One test batch overlapped an adapter reinstall and failed class loading. With
  resources held stable, the complete affected batches passed. Tests and final
  package verification use the same final adapter resources.
- Unsupported compiler/processor configurations retain diagnosed fallback;
  compiler-native fallback IDs can still be renumbered by javac.
