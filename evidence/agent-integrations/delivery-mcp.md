# Native MCP companion delivery

2026-11-12; TASK-52, TASK-53 and TASK-54. This records the current native MCP
route and supersedes the direct-terminal plugin route in
[the earlier TASK-52 record](delivery-task52.md). The public `ccnavigator`
terminal command remains available independently.

## Delivered boundary

Both host packages contain the same [canonical workflow](../../integrations/shared/graph-context.md),
host metadata, installation/removal guidance and license. The MCP adapter and
engine remain separately installed. The adapter invokes `ccnavigator` on PATH;
the host invokes `codecallnavigator-mcp` on PATH. Project/linked-root addresses
are explicit data configuration. No plugin code guesses a project, reads saved
graph storage, installs a server, implements graph queries, adds hooks or starts
analysis automatically. No graph/CLI/MCP contract extension was needed.

The package builder checks drift and strict metadata/allowlists, rejects
symlinks and unexpected server/hook files, assembles atomically into a new output
root and emits deterministic ZIP archives. Eleven filesystem/archive tests
include failure/retry and existing-output preservation. Twenty-one verification
planner tests cover the native MCP journey's position after independent MCP
installation and its PATH inputs.

## Compiler-backed native MCP

[Complete measured envelopes](native-mcp-protocol.json) retain 19 real SDK stdio
tool operations against the TypeScript compiler fixture. Temporary project and
dependency addresses are normalized; timings and response byte counts are from
the original run. Tool listing was also checked.

| Scenario | Observed result |
| --- | --- |
| Missing graph | `graph_missing`, `isError: true`; no graph artifact created |
| Discovery | Read-only TypeScript build target and default selection |
| Explicit refresh | Successful publication; initial freshness `unverifiable` with `unmapped_input` reasons retained |
| Ambiguous `start` | Two methods; details distinguish `entry.ts` from `other.ts` |
| Relationships | Outgoing `start` → `leaf`; incoming selection has no added caller |
| Tight bounds | One-node selection reports truncation and omitted nodes |
| Function flow | Authored method returns steps; body-less interface reports `flow_unavailable` and `isError: true` |
| Source edit | Status becomes `stale`; read queries preserve graph artifact bytes |
| Explicit refresh after edit | New generation published; old generation returns `generation_mismatch` |
| Unavailable executables | Missing CLI fails startup; missing MCP executable fails before invocation |

In this tiny fixture, non-refresh tool latency was 6.12–11.31 ms (median 7.63 ms).
The two explicit refreshes took 715.37 and 652.00 ms. These are local fixture
observations, not a large-project latency or token-cost claim. Raw tool responses
can exceed the skill's 8 KiB rendered-context budget; summaries must preserve
metadata and state omissions rather than pretending to reproduce every byte.

Reproduce after separately packaging the engine and MCP adapter:

```sh
export PATH="$PWD/dist/CodeCallNavigatorEngine/bin:$PWD/dist/CodeCallNavigatorMcp/bin:$PATH"
python3.13 tools/agent_mcp_journeys.py --node-modules /path/to/typescript/node_modules --output /tmp/native-mcp.json
python3.13 tools/package_agent_integrations.py --host all --check
python3.13 tools/package_agent_integrations.py --host all --output /tmp/new-agent-packages
```

The node_modules directory supplies TypeScript 6.0.3 and Node 24.15.0 was used.
Only the newly created fixture sources are analyzed. No production project graph
is refreshed by this recipe.

## Artifact identity

Canonical body SHA-256:
`49d7b3804302e3a006e156ebdc17502dda73965c09160af11375a6634008bf32`.

The isolated installation reports for both hosts measured the installed cached
skill SHA-256 as
`1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5`,
matching the generated canonical wrapper. Final isolated host tests use these
deterministic archives:

- Claude Code: `5b4d656eb9125b81dcf81550f6da904763748d86f6a8b3b74d00a416c53cd53f`
- Codex: `c29c881241eae62403616067253f77a53cfe0ce8ac5ddffd3773226834c0fce7`

## Host evidence and limits

Host conversations and lifecycle checks are recorded separately from protocol
acceptance. The [initial Claude transcript review](native-mcp-hosts-initial.md)
covers explicit/implicit native calls and disabled-MCP fallback. The extended
[Claude Code](claude-native-mcp.md) and [Codex](codex-native-mcp.md) host records
document repeat suppression, stale state, refresh and removal.

Both hosts passed explicit and implicit native invocation, actual `match`
status-only repetition, unavailable flow, stale/missing state, explicit refresh
and generation replacement, disabled-MCP source fallback, and isolated
disable/enable/removal. Unknown-binding clarification and an actually truncated
section were exercised separately; see each host record for exact outcomes.
Measured final responses stayed below 8 KiB. Ordinary query turns did not
refresh or change source/graph artifacts.

Targeted surfaces are local macOS arm64 Claude Code 2.1.306 and codex-cli
0.166.0, with normal authentication and isolated configuration. Desktop UI,
IDE/cloud/web surfaces, other platforms, minimum versions and optional hook
trust/disabled behavior remain unvalidated. No hooks are shipped. Claude CLI
configuration isolation does not disable its managed platform components;
Codex's authenticated managed plugins may coexist with the tested plugin.

## Review

Independent source review found no blocking defect or graph/API change. All
eight modified eligible Python source/test files reviewed at Code Health 10.0.
The final native-MCP pre-commit safeguard passed with eight eligible files checked
and `no-issues-found`; an empty findings array was accompanied by nonzero checked
coverage. Sonar was inactive because this repository has no `.sonar/config.json`.

Host-model limits are recorded rather than treated as backend behavior: one
Codex ambiguity response omitted the generation digest from its prose even
though native analyzed requests were correctly generation-bound; one Claude
response overparaphrased `unmapped_input` as an untracked source file. The exact
reason code remained present, and the backend may mean a dependency/toolchain
input. Successful host outcomes establish these measured conversations, not
universal model compliance.


## Full verification

`./verify-full.sh` passed **all 33 stages** at
`target/verification/agent-integrations-mcp-full`; the
[portable stage summary](native-mcp-verification.json) retains statuses and
elapsed times. The run used the configured isolated profiling snapshot, the
real private compiler acceptance fixture, and the installed TypeScript module
prerequisite. No benchmark ran against the live application store.

Passed: 552 ordinary Rust tests, 642 release Rust tests including all ignored
acceptance tests, 175 project Python tests, 26 tool Python tests and 112
TypeScript tests. Compilation, Clippy, storage/CPU/GPU acceptance and benchmarks,
app/engine packaging, signature verification, both deterministic plugin archives,
20 standalone CLI journey operations, independent MCP installation, 19 native
MCP journey operations, CodeScene and cleanup all passed. Full-run archive hashes
exactly match the final installed artifacts recorded above.

Markdown local links, protocol JSON syntax and the final diff were checked.
The two superseded interrupted full runs are not counted as successful checks;
the completed current native-MCP run is the delivery gate.
