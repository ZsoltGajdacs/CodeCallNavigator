# Initial native-MCP Claude host evidence

Independent read-only review on 2026-11-12 of the final-route traces under
`target/agent-integrations/claude-mcp-{explicit,implicit,disabled}.jsonl`.
These extracts intentionally omit credentials, session/request/tool-use identifiers,
model thinking/signatures, authentication metadata and machine-specific paths.
They are observations of these recorded conversations, not a claim that every
possible model response follows the workflow.

All three traces report Claude Code **2.1.306** and the installed
`codecallnavigator` plugin version **0.1.0**, with its graph-context skill listed.
The explicit and implicit cases report the native `codecallnavigator` server
connected. The disabled case reports no configured MCP servers.

## Observed tool counts

Counts include model tool calls, excluding protocol setup and tool responses.
Rendered output is the final user-facing response in UTF-8 bytes, not all tool
payloads or internal model context.

| Case | All tool calls | Native CodeCallNavigator MCP calls | Skill calls | Final response bytes |
| --- | ---: | ---: | ---: | ---: |
| explicit | 3 | 2 | 0 | 921 |
| implicit | 5 | 3 | 1 | 1397 |
| disabled | 4 | 0 | 0 | 1151 |

No case invoked `Bash`, terminal graph queries, analysis, `refresh_graph`, source
edits or configuration tools. Neither successful graph case exceeded the 8 KiB
rendered-response budget. These three cases do not exercise refresh, unsupported
flow, generation replacement, verified-fresh repeat suppression or uninstall;
those need separate evidence.

## Explicit-labelled ambiguity case

Observed sequence:

```text
ToolSearch(native graph tools)
graph_status({})
find_symbols({text: "start", category: "methods", offset: 0, limit: 20,
              expected_generation: <generation-from-status>})
```

Status and search both return the same version-1 CLI envelope generation.
Freshness is `unverifiable`, with reason `unmapped_input`; it is not relabeled
as match. The search has two hits and `more: false`. No symbol details, graph
section or flow query follows, so no candidate is silently selected.

Sanitized final-response extracts:

> I found two functions named `start`, so I need you to pick one before I look up its callees.

The response distinguishes `entry.ts` (`start(flag: boolean): number`) from
`other.ts` (`start(): number`), reports unverifiable freshness and its reason,
and asks which candidate to investigate. The trace lists the installed skill;
it does not contain a separate `Skill` tool call in this explicit-labelled run.
The launch command, rather than a fabricated Skill event, establishes explicit
invocation.

## Implicit-selection caller case

Observed sequence:

```text
Skill({skill: "codecallnavigator:graph-context"})
ToolSearch(native graph tools)
graph_status({})
find_symbols({text: "leaf", category: "methods", offset: 0, limit: 20,
              expected_generation: <generation-from-status>})
graph_section({selection: {kind: "neighborhood", symbols: [<returned-leaf-id>],
                           direction: "Incoming", depth: 1},
               limits: {nodes: 100, edges: 300},
               expected_generation: <generation-from-status>})
```

The native section succeeds and returns one direct call relationship from
`entry.ts` start to leaf, provenance `typescript.compiler`, no truncated results,
and the same generation/freshness as the earlier status. The response records
server binding, query direction, bounds, default filters, source locations,
certainty and provenance. It explicitly says structural presence does not prove
that the call always runs.

Sanitized final-response extracts:

> According to the CodeCallNavigator graph, **`leaf` has one caller: `start`**.

> **The graph may not match the current code.**

Observed limitation: the model paraphrases `unmapped_input` as a source file not
tracked. The envelope only establishes an unmapped observed input, which may be
a dependency/toolchain input rather than a project source file. The precise
reason code is preserved; the paraphrase should not be treated as new backend
semantics or proof of a particular untracked source file.

## Disabled-MCP source fallback

Observed sequence:

```text
ToolSearch("graph_status codecallnavigator") -> no matching tools
Grep({pattern: <leaf-word-search>, glob: "!**/.codecallnavigator/**",
      output_mode: "content", path: <temporary-project>})
Glob({pattern: "**/*", path: <temporary-project>})
Read({file_path: <temporary-project>/other.ts})
```

No native graph calls, terminal commands, saved-graph content reads, analysis or
configuration changes occur. Grep explicitly excludes saved graph storage and
returns numbered `entry.ts` source lines. Read accesses only `other.ts`. Glob
lists source/configuration files **and saved graph/cache filenames**; this is
filename enumeration, not reading those artifacts' contents. Do not describe
this trace as proof that artifact paths were never enumerated.

Sanitized final-response extract:

> I couldn't run a graph query: no CodeCallNavigator MCP tools are set up in this session.

The response labels the answer as source-text evidence, cites `entry.ts:1` and
`entry.ts:2`, distinguishes the unrelated `other.ts` start, and acknowledges
that text search cannot establish all indirect call relationships.

## Final source wrapper consistency

The current repository wrappers were independently checked with the package
validator and direct byte comparison against its generated header plus the
canonical body. This is static package consistency evidence; it does not prove
which exact source bytes were present at an earlier trace timestamp.

Canonical body SHA-256: `49d7b3804302e3a006e156ebdc17502dda73965c09160af11375a6634008bf32`.

| Host source | Result | Current wrapper SHA-256 |
| --- | --- | --- |
| claude-code | exact generated wrapper match | `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5` |
| codex | exact generated wrapper match | `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5` |
