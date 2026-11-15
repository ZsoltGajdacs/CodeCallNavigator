# TASK-22.7 acceptance evidence

The delivered API is shared-core `flow::expand`. CLI and desktop expansion
controls remain TASK-22.8. Existing single-function and comparison workflows
retain their behavior.

| Criterion | Proof |
| --- | --- |
| 1. Exact occurrence, source and relationship ownership | [Result contracts](unit3/README.md), [assembly](unit4b/README.md), [nested occurrences](unit5a/README.md), and [real compiler consumers](unit6/README.md). Structural IR paths, call slots, cleanup routes and complete targets distinguish occurrences; source bytes and digests remain owner-bound. |
| 2. Supported synchronous call/return and completion | [Builder refactor](unit4a/README.md) and [assembly](unit4b/README.md) cover caller continuation, callee and caller cleanup, exceptions, returns and unsupported transfers. [Compiler consumers](unit6/README.md) prove actual SDK facts reach expansion through cached artifacts. |
| 3. Active-path cycles, independent repeats and explicit choices | [Nested occurrence matrix](unit5a/README.md) covers direct/mutual recursion, repeated siblings, diamonds, candidate selection, request permutation and blocked descendants. |
| 4. Validated bounds without silent truncation | [Budget matrix](unit5b/README.md) covers exact and one-under limits, late caller/cleanup work, deterministic repeated demotion, source-owner deduplication, oversized children, mandatory-root failure and cancellation. |
| 5. Conservative unsupported execution | [Assembly](unit4b/README.md), [nested matrix](unit5a/README.md), and [final negative matrix](unit6/README.md) cover callbacks/deferred regions, references, HTTP/navigation, async/deferred bodies, missing bodies and exceptions/cleanup. Full choices survive visible stops; no child body or unused child source is invented. |
| 6. Compatibility and immutable inputs | [Invocation contracts](unit1/README.md) retain legacy serialization; [ordinary chart goldens](unit4a/README.md) remain exact. [Compiler consumers](unit6/README.md) assert unchanged cached artifact bytes. Expansion does not modify input graph/IR inventories. |

The [final full report](final/README.md) records current verification after the
two lint cleanups and the independently reproduced [publication-lock lifetime
fix](lock-lifetime/README.md). Earlier checkpoints are historical evidence, not
substitutes for the final full run. All category scripts use the new inventory
and owned-workspace cleanup paths; no blanket serialization or shortened recovery
timeouts were introduced. The final full run selects all 1,447 registered cases
and eleven standalone gates, with no registered cases or gates left unrun.

CodeScene reviews and safeguards are retained with each implementation unit.
The final full run also checks the complete changed source set. SonarMCP is
inactive because this checkout has no `.sonar/config.json`. No pull request was
opened, so the before-PR change-set check does not apply.
