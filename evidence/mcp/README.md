# CLI-only MCP acceptance

The separate `codecallnavigator-mcp` package uses the official `rmcp` SDK and has
no core, engine or desktop dependency, including test dependencies. Runtime code
accesses graph/source information only through the configured CLI executable.
See [installation and public tools](../../docs/mcp.md).

## Process acceptance

Package integration tests speak real MCP JSON-RPC over stdio, without importing
application libraries. A real TypeScript compiler fixture exercises explicit
refresh, same-named symbol ambiguity, details/source/function flow/neighbors,
bounded selections and exact response comparison with direct CLI queries. It
also covers linked-root explorer access before analysis, missing graphs/symbols,
live versus analyzed source, staleness and generation mismatches.

Real Gradle cancellation/disconnect fixtures first publish a compiler-produced
graph, then stop a wrapper and its child during another refresh. Tests check that
the previous artifact remains byte-identical, cancellation emits no late MCP
response, and another generation-bound query succeeds. Narrow fake CLI fixtures
cover incompatible capabilities, malformed response metadata and hung queries.

Focused process regressions cover blocked writes, dropped request futures,
stderr backpressure before the CLI hello and independently grouped descendants.
Unexpected CLI crashes can orphan processes before ownership is observable; that
case reports unverified descendant cleanup instead of promising complete recovery.

## Independent installation

`tools/package_mcp.py` creates the MCP's own Cargo archive, extracts it outside
the repository, builds/installs it into a temporary prefix, verifies its native
signature/help, and runs real stdio against the separately installed CLI. The
result contains only the MCP binary, guide and license notices; it bundles no
engine, analyzers or desktop resources. The source `.crate` is retained alongside
the binary archive.

An initial check exposed stale Cargo reuse with normalized archive timestamps;
installation now overrides both Cargo build directories with a fresh temporary
directory. Complete installation/package/smoke passed afterward. Boundary checks
also reject transitive test/target dependencies on application packages and
source inclusion in runtime/tests, while permitting test fixture filesystem use.

Full verification runs under `target/verification/task50-full-20261112-r3/`.
The earlier `task50-full-20261112` run passed 545 ordinary Rust tests, then stopped
on an existing verification fixture's 300 ms interpreter-startup timeout before
its readiness PID existed. The same failure reproduced in its focused suite.
The fixture now allows one second while retaining the three-second cleanup bound,
30-second hanging process and both stopped-PID assertions; all 16 focused tests
pass. Original failure evidence remains intact.

The second run, `task50-full-final-20261112`, caught a separate readiness race:
a desktop fake CLI exposed a newly created PID file before writing its contents.
Test fixtures now publish completed PID files atomically in both desktop and MCP
process tests and in the Python quality fixture. Seven focused desktop query
tests and the 16 quality tests pass, and independent review confirms the original
cancellation/restart/process-death assertions remain intact.

All 44 stages passed in the final run: 546 ordinary Rust tests, 636 release tests
including ignored acceptance, 164 Python project tests, 26 Python tool tests and
112 TypeScript tests. Storage targets, CPU/GPU benchmarks, app/engine packaging,
signature checks and MCP independent archive installation/stdio smoke passed.
CodeScene reviewed all 23 eligible changed source files without findings; its
23/23 safeguard passed without regressions. Final cleanup passed as well.

The Backlog task records the chronological action log. Full checks include the
independent installation stage and use an isolated profiling copy; Sonar is
inactive because the project has no `.sonar/config.json`.
