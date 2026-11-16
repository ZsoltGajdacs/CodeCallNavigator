# MCP adapter practices and testing

Follow [root guidance](../../AGENTS.md) and [the testing strategy](<../../.backlog/docs/testing/doc-12 - Testing-strategy.md>).

MCP communicates only with the installed CLI. Do not depend on core, engine or
desktop libraries, including in tests; do not read project graphs directly or
reimplement graph algorithms in the adapter.

Prefer unit tests for argument validation, wire shapes, bounds, metadata and error
translation. Use real protocol values with explicit inputs and observable results.
Use integration tests for stdio framing, subprocess ownership, deadlines and
cancellation. Narrow documented fault probes may generate malformed frames,
blocked pipes or stalled children; they must not invent successful business results.
Keep representative end-to-end tests using a real MCP client, adapter executable
and installed CLI. Verify generation/freshness and error fidelity, independently
installed packaging, and preservation of the prior artifact after failure.

```sh
./verify-changes.sh
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Run from the workspace root. Use the report-only command to inspect CLI and language-toolchain
requirements. Real analysis journeys need the corresponding compiler resources;
protocol checks alone do not establish those journeys. Category scripts clean
owned Rust builds and temporary resources on success and failure while retaining
evidence; use `--keep-artifacts` only for explicit diagnostics.
Every expensive test needs a boundary-risk rationale. Remove a redundant test only
with a retained-assertion mapping and demonstrated defect detection. Roughly one
minute of warm unit execution is an aspiration, not a gate or coverage target.

Agents must use `./verify-changes.sh` for verification selection and
`./verify-changes.sh --run` for all test and verification execution. Do not
substitute raw test commands or manually selected category/component scripts.
Execute the complete computed plan; full verification is selected automatically
by checked-in rules, with no manual scope override. Use `--base REF` to include
committed changes. Run automatically selected full verification outside the macOS
sandbox. Report unavailable prerequisites as not run, never as passing checks.
