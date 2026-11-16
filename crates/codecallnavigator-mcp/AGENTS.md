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
./verify-mcp.sh --suite unit
./verify-integration.sh --component mcp
./verify-acceptance.sh --component mcp
```

Run from the workspace root. Use `--dry-run` to inspect CLI and language-toolchain
requirements. Real analysis journeys need the corresponding compiler resources;
protocol checks alone do not establish those journeys. Category scripts clean
owned Rust builds and temporary resources on success and failure while retaining
evidence; use `--keep-artifacts` only for explicit diagnostics.
Every expensive test needs a boundary-risk rationale. Remove a redundant test only
with a retained-assertion mapping and demonstrated defect detection. Roughly one
minute of warm unit execution is an aspiration, not a gate or coverage target.
