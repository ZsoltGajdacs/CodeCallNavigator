# Shared graph-context acceptance journeys

These journeys apply to both host packages. The canonical workflow is
[graph-context.md](graph-context.md). Backend protocol assertions do not prove
skill selection, ambiguity handling, rendered limits or repeated-context
suppression; those require recorded real host conversations with native MCP.
Previous direct-CLI conversation evidence proves only the earlier route.

## Real compiler and MCP baseline

Install matching `ccnavigator` and `codecallnavigator-mcp` commands on PATH,
with the engine's analyzer resources intact, plus Node 24+/TypeScript 6.0.x.
From the repository root:

```sh
python3.13 tools/agent_mcp_journeys.py --node-modules /absolute/path/node_modules --output target/agent-integrations/mcp-backend.json
```

The command creates an isolated TypeScript project with two exported `start`
methods, a `leaf` callee and a body-less `run` method. A project-bound real SDK
MCP server runs `ccnavigator` through its normal subprocess boundary. The test
asserts missing graph/no publication, ambiguity, directed bounded sections,
`isError`/`flow_unavailable`, stale inputs, read-only artifact preservation,
explicit refresh, generation mismatch and unavailable CLI/MCP. It records real
initial freshness (`match` or `unverifiable`), never substitutes a fabricated
fresh envelope. `--engine` and `--mcp` are optional executable test overrides;
ordinary plugin setup uses command names on PATH, not binary paths.

The independent `tools/agent_journeys.py` test remains standalone `ccnavigator`
compatibility coverage. It is not the current plugin route and does not prove
native MCP integration.

## Host setup and conversations

Use isolated host settings, the same compiler fixture and a separately
configured MCP server explicitly bound to its project root. Tell the host the
server-to-project mapping or expose that trusted configuration. Workspace path,
server name and status generation alone do not prove the binding; an unknown
binding must cause a clarification before graph queries. Install the real
package through its normal local plugin path. Record host version, skill
selection, native MCP calls and final output. Keep normal trust and approvals;
never copy credentials into evidence or enable blanket refresh approval.

| Journey | Prompt / preparation | Required observable result |
| --- | --- | --- |
| Explicit invocation | Invoke graph-context: “Explain possible callees of start.” with known project binding | Skill loads; native graph_status then generation-bound find_symbols limited to 20; both starts presented by distinguishing location, no silent choice or refresh |
| Implicit selection | New conversation: “Use the CodeCallNavigator graph to explain possible callers of leaf.” | Installed skill selected without explicit mention; status then bounded Incoming graph_section; output preserves generation/freshness and uncertainty |
| Unknown binding | Provide tools but no known mapping to the intended project | Ask for the mapping before graph queries; no inference from generation or names |
| Resolve ambiguity | Reply “Use entry.ts start.” | symbol_details confirms candidate; Outgoing depth 1 graph_section at 100/300 limits contains leaf; generation bound consistently |
| Function flow | “Explain flow of Contract.run.” | Native function_flow returns isError/flow_unavailable; no invented body or execution |
| Repeat unchanged | With real match status, repeat the identical caller question in one conversation | Second graph_status verifies same generation; reference prior summary without another section call or repeated graph content |
| Unverifiable repeat | Repeat when real status is unverifiable | Preserve reasons and uncertainty; do not claim verified-current reuse; this does not prove the match repeat path |
| Before/after edit | Ask impact of editing leaf; externally edit it; ask again | Bounded incoming context before edit; native status after edit says stale; prior analyzed evidence labeled and refresh offered, not run |
| Explicit refresh | “Refresh this fixture graph now, then explain callers of leaf.” | Native discover_builds/refresh_graph only after request; status and symbol resolution restart; no old-generation answer mixed in |
| Missing graph | New empty project, ask for leaf callers | graph_missing explained; refresh offered, not called; queries do not create artifact |
| Disabled/unavailable MCP | Remove/disable server or break configured CLI in isolated settings | Limitation explained once; source fallback excludes .codecallnavigator; no terminal graph-query fallback, auto-install or shell analysis |
| Context limit | Request a section large enough to truncate or a smaller budget | At most 8 KiB rendered context, metadata/omission markers preserved, no completeness claim |
| Removal | Uninstall package and separately remove its user-configured MCP registration | New session has neither skill nor registered tools; installed engine/MCP binaries, source and graph artifacts unchanged |

Unsupported surfaces, unavailable authentication and unexercised paths remain
unverified. If the real fixture cannot report match, keep that repeat path
pending rather than relabeling unverifiable or simulating success.
