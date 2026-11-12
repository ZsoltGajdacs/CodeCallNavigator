# CLI-backed MCP

`codecallnavigator-mcp` is an independently installable Rust executable using the
[official Rust MCP SDK](https://docs.rs/rmcp/3.6.0/rmcp/) and stdio transport.
Install the matching engine distribution with `ccnavigator` on PATH, then install the
adapter with `cargo install --locked --path crates/codecallnavigator-mcp`.
The package has no dependency on the core, engine or desktop libraries, including
in tests. Its only runtime graph interface is the configured CLI process.

To produce and verify the independent binary/source packages locally:

```sh
python3.13 tools/package_mcp.py --engine /absolute/path/codecallnavigator-engine
```

The binary is placed at `dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp`,
alongside a guide and license notices. The ZIP and source `.crate` are in `dist/`.
The package contains no engine or analyzer resources.

Start an MCP client with this command and argument array:

```json
{
  "command": "codecallnavigator-mcp",
  "args": ["--engine", "ccnavigator", "--project", "/absolute/path/project", "--root", "linked=/absolute/path/linked"]
}
```

The runtime example resolves both executables from PATH. Keep `ccnavigator`
installed with its analyzer resources; only project/root addresses need explicit
paths. The absolute engine path in the build verification command is a test input,
not required runtime configuration.

`--engine` and `--project` are required and unique. Repeat `--root NAME=PATH` for
linked roots; names must be unique and `repo` is reserved for the project. These
addresses remain fixed for the server lifetime. Configuration parsing performs
no project filesystem reads; canonicalization, discovery, source confinement and
artifact/database access belong to the CLI. Relative addresses follow the CLI's
normal working-directory/project rules; absolute paths avoid client ambiguity.

Startup requires CLI query contract version 1, generation binding, and every
advertised operation used by the tools. Incompatible capabilities or a failed
capabilities process prevent startup. MCP negotiation is handled by the SDK;
stdout carries MCP messages only and startup failures go to stderr.

## Tools and arguments

All argument objects reject unknown fields. Related analyzed queries can carry
`expected_generation`, a 64-character hexadecimal CLI digest. A generation
mismatch is an explicit CLI error; request status again before combining results.
Queries never start analysis. Only `refresh_graph` changes analyzed data.

| Tool | Arguments |
| --- | --- |
| `graph_status` | `expected_generation?` |
| `find_symbols` | `text`, `category?` (`all`, `classes`, `methods`), `offset?` (0), `limit?` (50; 1–200), `expected_generation?` |
| `symbol_details` | `symbol`, `expected_generation?` |
| `graph_section` | `selection`, `filter?`, `limits?`, `expected_generation?` |
| `neighbors` | `symbol`, `filter?`, `limits?`, `expected_generation?` |
| `symbol_source` | `symbol`, `origin?` (`analyzed` or `live`, default analyzed), `expected_generation?` |
| `function_flow` | `symbol`, `expected_generation?` |
| `explore_directory` | `path`, `offset?` (0), `limit?` (1000; 1–1000), `expected_generation?` |
| `discover_builds` | empty object |
| `refresh_graph` | `targets?`: array of `{path, kind}` identities returned by `discover_builds` |

Search text and identities/paths are limited to 16 KiB. Graph limits require both
`nodes` (1–1500) and `edges` (1–6000). Neighborhood roots require 1–1500 stable
symbol IDs. Traversal depth is 0–64. The explorer tool deliberately caps each page
at 1000 entries; callers use returned pagination metadata for later pages.

Selections use the CLI's tagged objects:

```json
{"kind":"overview"}
{"kind":"entry_points","direction":"Outgoing","depth":2}
{"kind":"neighborhood","symbols":["stable-symbol-id"],"direction":"Both","depth":1}
{"kind":"path","from":"stable-a","to":"stable-b"}
{"kind":"file","file":"CLI-returned-file"}
{"kind":"class","build":"CLI-returned-build","project":"CLI-returned-project","container":"ClassName"}
```

Explicit relationship filters require version 2 and both independent arrays;
empty arrays preserve the caller's selection rather than applying defaults:

```json
{
  "version": 2,
  "include_tests": false,
  "include_generated": false,
  "include_unavailable": false,
  "relationship_kinds": ["call", "function_reference"],
  "target_resolutions": ["direct", "possible"]
}
```

Relationship kinds also include `framework_lifecycle`, `http_navigation`, and
`other`; resolutions also include `virtual_dispatch` and `other`. Omitting the
filter delegates defaults to the CLI. The adapter validates shapes/bounds only;
search, traversal, class membership, certainty and flow support remain CLI-owned.

## Responses, refresh and cancellation

A graph tool's `structuredContent` is the complete CLI query envelope, unchanged,
including generation, freshness, provenance, diagnostic and truncation metadata.
A short text content block accompanies it. CLI error envelopes set MCP `isError`
and retain the original error code/message. Invalid arguments use
`invalid_arguments`; process, malformed-wire and deadline failures use
`cli_transport_error`. Unknown tools are MCP method-not-found errors. Discovery
returns the CLI's complete discovery object. No snapshot/render/inspector tools or
MCP-side graph algorithms/cache exist.

Refresh first runs CLI discovery. Omitted targets select exactly the CLI's
`default_selection` identities; explicit identities must match discovered DTOs.
The adapter never computes build-selection policy. It validates the `serve-once`
version-1 hello before sending `analyze`, sets `infer_linked_roots=false`, supplies
only configured linked roots, and keeps stdin open during analysis. Progress and
exactly one terminal event must be well formed. Success additionally requires a
successful process exit. Completion is wrapped in a version-1 envelope with the
publication digest as `generation` and the full completion event as `result`;
failed analysis returns `analysis_failed` and preserves its original `cli_event`.

Each operation starts one CLI subprocess. Async reads/writes observe cancellation
and deadlines, including blocked stdin writes. Requests are limited to 4 MiB,
stdout to 32 MiB, and captured stderr to 64 KiB (excess stderr is drained). Query
and discovery deadlines are 120 seconds, capability discovery 15 seconds, and
explicit refresh 30 minutes. A too-large response fails explicitly.

MCP cancellation, dropped request futures and client disconnect cancel owned
supervisors. Analysis receives cooperative EOF cancellation with a five-second
grace period; queries terminate immediately. Forced termination/reaping is bounded
by another five seconds. Unix subprocess groups and bounded live OS PID/PPID/PGID
inventories support analysis cleanup, including descendants that establish
separate analyzer groups. Query errors do not require process-table inspection.
Unavailable inspection and analysis exits before live inventory are explicitly
reported as unverified cleanup. An abrupt CLI crash can orphan a newly spawned
independent group before it can be observed; the adapter cannot guarantee cleanup
in that case and reports this uncertainty. It does not retain a long-running
cache of process IDs that could be reused by unrelated processes.
Server shutdown waits for all owned supervisors before leaving the async runtime.

## Verification

`cargo test --locked -p codecallnavigator-mcp` covers configuration, argument/wire
validation and real subprocess transport lifecycle. Process-only integration
acceptance under this package's `tests/` invokes the real CLI and compiler
fixtures without importing workspace libraries. The workspace verification runner
also enforces the dependency/source boundary and installs the packaged crate
outside the workspace before an MCP stdio smoke test.
