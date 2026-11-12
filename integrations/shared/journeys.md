# Shared graph-context acceptance journeys

These inputs and expected outcomes apply to both host packages. The canonical
workflow is [graph-context.md](graph-context.md). A backend test proves protocol
facts; only a recorded real host conversation can prove skill selection,
ambiguity handling, rendered context limits, or repeated-context suppression.
Do not count a configuration/manifest check as model acceptance.

## Real compiler and CLI baseline

From the repository root, with separately installed matching CLI and Node
24+/TypeScript 6.0.x dependencies:

```sh
python3.13 tools/agent_journeys.py --node-modules /absolute/path/node_modules --output target/agent-integrations/backend.json
```

The executable default is `ccnavigator` on PATH. `--engine` can override it with an
executable name or path when testing a separate build; users need not configure
binary paths when their installed commands are on PATH.

The default command name is the user-installed `ccnavigator`; no MCP server or
binary-path configuration is required.

The command creates an isolated TypeScript project with two exported `start`
methods, one `leaf` callee and a body-less interface `run` method. It invokes only
the public CLI JSON query contract, records requests/responses/timing, and asserts missing graph
does not publish, same-name ambiguity survives, caller/callee directions are
distinct, tight bounds report truncation, unsupported flow is explicit, edited
inputs become stale, queries leave artifact bytes unchanged, explicit refresh
replaces generation, and an unavailable CLI fails. It records the initial
freshness rather than assuming `match`; unmapped dependency inputs can produce
`unverifiable`. No artificial fresh envelope is substituted.

## Host conversations

Use an isolated host configuration and the same fixture, with the project root
explicitly passed in every query. Install the package through the host's
normal local plugin path and record host version, invocation, CLI calls and
user-visible output. Do not bypass trust or copy credentials into evidence.

| Journey | Prompt / preparation | Required observable result |
| --- | --- | --- |
| Explicit invocation | Invoke the host's graph-context skill: “Explain possible callees of start.” | Skill loads, capabilities/status precede analyzed queries, search is generation-bound and limited to 20; two starts are presented with distinguishing locations; no first-hit selection and no analysis |
| Implicit selection | In a new conversation: “Use the CodeCallNavigator graph to explain possible callers of leaf.” | Host selects the installed skill without explicit mention; status then bounded Incoming selection; output identifies generation/freshness and possible relationships |
| Resolve ambiguity | Respond “Use entry.ts start.” | Method details confirm location; Outgoing section at depth 1 and 100/300 limits contains leaf; all analyzed calls bind the checked generation |
| Function flow | “Explain flow of Contract.run.” | `flow_unavailable` is retained; host explains no source body rather than inventing flow |
| Repeat unchanged | With real status `match`, ask the identical caller question twice in one conversation | Second status confirms same generation; host refers to previous summary without repeating the section request or graph content. Changed scope is a different request |
| Unverifiable repeat | If real status is `unverifiable`, repeat the question | Host labels uncertainty, preserves reasons and does not claim verified-fresh reuse; this does not prove the `match` repeat path |
| Before/after edit | Ask impact of editing leaf; edit its return value externally, then ask again | Bounded incoming context before editing; status after edit reports stale; host labels prior analyzed evidence and offers analysis without running it |
| Explicit refresh | “Refresh this fixture graph now, then explain callers of leaf.” | Discovery/analysis only after request; status and symbol resolution restart; no old-generation answers mixed in |
| Missing graph | New empty fixture, “Explain callers of leaf using the graph.” | `graph_missing` explained, analysis offered but not run; no artifact created by queries |
| Unavailable CLI | Remove ccnavigator from the isolated PATH, or disable terminal access | Explain unavailable structural evidence once; source exploration can continue; no auto-install, MCP fallback or implicit analysis |
| Context limit | Ask for a section large enough to truncate, or set a smaller requested budget | Visible generation/freshness/provenance and omission markers; rendered graph context is at most 8 KiB; no completeness claim |
| Removal | Uninstall plugin | New conversation has neither installed skill nor package instructions; the independently installed CLI, source and published artifacts remain unchanged |

Record unsupported surfaces, missing authentication, and unexercised paths as
unverified. If the real fixture cannot report `match`, the verified-fresh repeat
path stays pending; do not relabel `unverifiable` or simulate its success.
