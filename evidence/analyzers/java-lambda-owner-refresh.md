# Java lambda owner refresh regression (TASK-33)

## Cause and correction

During processor-disabled javac attribution, an unresolved generated-getter method
reference after a stream filter can make an expression resolve to an error
TypeElement named `java.util.stream.Stream$map`. Treating any ancestor TypeElement
as the lambda owner assigned lambdas from two TaxReduction files to that type,
triggering the strict Rust cross-file check.

`LambdaOwner` now resolves enclosing class declarations only. Correlation sites,
labels, label contexts and identity eligibility share declaration-only checks.
Strict cross-file validation remains and reports the owner and both paths.
Inventory cache keys include the adapter JAR content hash; the wire format remains
`kind_v8` and source inventory version 8.

## Regression-first evidence

A simple map lambda did not reproduce. The real-JVM fixture compiling two nested
classes with getters, then removing those getters before source inventory,
reproduced the failure when a filter lambda was followed by `map(Item::getValue)`.
Before the fix, its owner assertion failed with `java.util.stream.Stream$map`
instead of `fixture.First$Nested` and `fixture.Second$Nested`.
After the fix, both owners and labels are lexical, with unavailable replay reasons
rather than invented compiled/source bindings. Rust end-to-end coverage verifies
inventory loading, graph validation, file ownership, readable labels and diagnostics.
Valid stream lambdas and local/anonymous type fallback have real-JVM coverage too.

## Verification

- Adapter Gradle offline `installDist`: passed with existing Gradle cache access.
- `cargo test --workspace --locked`: all unignored tests passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Python Java lambda correlation: 12 passed; Java context: 9 passed.
- Python packaging: 10 passed.
- Rust unresolved-stream normalized-graph regression: passed.
- Rust cache invalidation and strict correlation boundary tests: passed.
- Java correlation and identity integration acceptance: 4 + 3 passed.
- Packaged Java resources acceptance: 2 passed; existing Gradle cache access required.
- Full `python3 tools/package.py`: passed, including desktop signing verification.
- Development, standalone engine and desktop adapter JAR SHA-256 match:
  `524f507199d2d0d05fe8eee2084444df99c37dc21d37eff3272ac32bc23a236e`.
- Actual packaged desktop engine process, using its bundled resources: a stream
  filter/map fixture produced three nodes and two portable lambda identities.
- Independent reviewer: no actionable findings.
- CodeScene: all 9 modified/new eligible source files score 10.0; pre-commit
  safeguard passed 9/9 eligible files, with no regressions.
- No `.sonar/config.json`; SonarMCP is inactive.

## Original project retry

Used the saved successful Maven metadata with the existing
`recurring_billing_successful_build_outputs_preserve_both_generated_copies`
acceptance test; no application build or application-source edits were performed.
All 13 compiled contexts completed: **10,827 declarations, 17,331 relationships**.
The analyzer checks source and bytecode snapshots for changes during this run.
Both TaxReduction lambda owners now refer to their respective ArticleType classes.
Compiler replay remains unavailable where generated getters cannot be attributed,
with reason `Source attribution prevents compiler replay`.

Ephemeral detailed outputs: `/tmp/codegraph-task33-billing.json`,
`/tmp/codegraph-task33-core.tsv`, and `/tmp/codegraph-task33-packaged.json`.
Planning, exploration and implementation ran in the main agent; independent review
and integration acceptance testing ran in separate reviewer/tester agents.
