# Claude Code native MCP acceptance

2026-11-12, macOS arm64, Claude Code 2.1.306. Actual model sessions used the installed plugin's cached skill and the separately installed Rust SDK MCP server over stdio. No shell MCP shim or direct CLI query wrapper was used by the model. Engine commands are resolved through PATH: `codecallnavigator-mcp --engine ccnavigator --project PROJECT`.

This document retains sanitized native tool inputs, selected result fields, output excerpts and byte counts. Tool-result excerpts are deliberately reduced evidence, not a replacement MCP wire contract; exact full tool envelopes were checked independently by `tools/agent_mcp_journeys.py`. Stable symbol strings are consistently shortened to `S1`, `S2`, etc. Temporary fixture/install and dependency locations are placeholders. No authentication data, hidden reasoning or message signatures are retained here.

## Reproduction recipe

Prerequisites: matching installed `ccnavigator` and `codecallnavigator-mcp` on PATH, Node 24+/TypeScript 6.0.x modules, an authenticated Claude Code session, and the packaged Claude plugin from `tools/package_agent_integrations.py`. Install the package into a temporary `CLAUDE_CONFIG_DIR` using its local marketplace, and use that installed cached plugin directory for the session. Existing Claude authentication is used separately; do not copy credential files into the fixture or evidence.

Create a temporary project from `tools.agent_journeys.fixture` (`entry.ts` exports `leaf` and `start`, `other.ts` exports another `start`, `Contract.run` has no body). Analyze only that project's `tsconfig.json`. For the real `match`-freshness case only, explicitly configure `--root observed-inputs=/` on the analysis and server. This maps the compiler-observed ancestor manifests/dependencies into portable provenance; it does not request a filesystem-root scan. The mapping is a broad **test-only fixture setting**, never a product installation default. Default-root `unverifiable` behavior is covered independently in the backend and initial implicit host cases.

Example temporary MCP JSON (absolute project address, no binary paths):

```json
{"mcpServers":{"codecallnavigator":{"type":"stdio","command":"codecallnavigator-mcp","args":["--engine","ccnavigator","--project","PROJECT","--root","observed-inputs=/"]}}}
```

Run Claude in the temporary project with `--print --setting-sources '' --strict-mcp-config --mcp-config FILE --no-session-persistence --plugin-dir INSTALLED_CACHE --tools=default --input-format=stream-json --output-format=stream-json --verbose`. Permit the requested read tools (`Skill`, `ToolSearch`, native graph tools and ordinary source readers); the explicit-refresh turn additionally authorizes `refresh_graph`. The recorded runs used explicit tool allowances and `--permission-mode=dontAsk`; no bypass-trust or bypass-permission flag was used. Read existing account credentials outside the sandbox through normal Claude authentication. Send each prompt as a JSON-line `{"type":"user","message":{"role":"user","content":"PROMPT"}}`, wait for its `type: result`, then send the next prompt to the same process.

Prompts, in order:

1. Explicitly invoke `/codecallnavigator:graph-context`, identify the configured server/project and test-only mapping, ask possible callers of `leaf`, and prohibit refresh/edits.
2. Ask the identical caller question with the same project/server, generation, direction, filters and bounds; prohibit refresh/edits.
3. Ask the function flow of `Contract.run`; prohibit refresh/edits.
4. Externally change only `other.ts`'s return value to 45, then ask possible impact of editing `leaf`; prohibit refresh/edits.
5. Explicitly authorize `refresh_graph` for this fixture only, then ask possible callers of `leaf`; prohibit source edits.

A separate empty-project server checks `graph_missing`; a separate new session after uninstall/catalog/server removal uses an empty MCP configuration and no plugin directory. Full transient logs: `target/agent-integrations/claude-mcp-conversation-final.jsonl`, `claude-mcp-missing-final.jsonl`, `claude-mcp-after-removal-final.jsonl`, and lifecycle report `claude-mcp-final-tester.json`.

## Observed results

| Turn | Native graph tools | Output bytes | Outcome |
| --- | --- | ---: | --- |
| Real match callers | graph_status, find_symbols, graph_section | 1429 | Recorded below |
| Same-session repeat | graph_status | 605 | No repeated section |
| Unavailable body flow | graph_status, find_symbols, function_flow | 1125 | Recorded below |
| Stale after external edit | graph_status, graph_section | 2278 | Recorded below |
| Explicit refresh and new generation | refresh_graph, graph_status, find_symbols, graph_section | 1928 | Recorded below |

Every analyzed query binds `expected_generation`; ordinary turns do not invoke refresh. The repeat turn performs exactly one native `graph_status`, keeps the same real `match` generation, and refers to the prior caller summary without a graph-section query or repeated caller content. Stale impact is labeled as earlier analyzed evidence and kept separate from current-source observations. Only the explicitly authorized fifth turn calls `refresh_graph`, publishes a new generation, requests status, resolves the method again, then queries the new generation.

Claude exposes successful native tool results as full JSON envelopes. For `isError` tool results it renders the server's text block (for example `CLI error flow_unavailable: No source body is available for this declaration.`) with `is_error: true`; this host transcript does not expose structured error generation/freshness metadata. The model retained the error code/message and used the preceding status check for graph context instead of inventing a flow.

### Turn 1: Real match callers

```json
[
  {
    "tool": "graph_status",
    "input": {},
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 6,
        "edges": 1,
        "diagnostics": [],
        "diagnostics_total": 0
      }
    }
  },
  {
    "tool": "find_symbols",
    "input": {
      "text": "leaf",
      "category": "methods",
      "offset": 0,
      "limit": 20,
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "hits": [
          {
            "name": "leaf",
            "detail": "(): number · project · <isolated-fixture>/project/tsconfig.json",
            "target": {
              "kind": "method",
              "symbol": "S1"
            }
          }
        ],
        "offset": 0,
        "limit": 20
      }
    }
  },
  {
    "tool": "graph_section",
    "input": {
      "selection": {
        "kind": "neighborhood",
        "symbols": [
          "S1"
        ],
        "direction": "Incoming",
        "depth": 1
      },
      "limits": {
        "nodes": 100,
        "edges": 300
      },
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 2,
        "edges": 1,
        "truncated": false,
        "omitted_nodes": 0,
        "omitted_edges": 0,
        "diagnostics": [],
        "diagnostics_total": 0,
        "roots": [
          "S1"
        ],
        "relationships": [
          {
            "from": "S2",
            "to": "S1",
            "kind": "call",
            "certainty": "direct",
            "provenance": "typescript.compiler",
            "classification": {
              "kind": "call",
              "resolution": "direct"
            }
          }
        ]
      }
    }
  }
]
```

Observed response:

The graph shows one caller of `leaf`: the function `start` in `entry.ts`.

**Setup:** I used the `codecallnavigator` MCP server bound to this project, with the root mapping you gave (`observed-inputs=/`). Generation is `1c053c1829d1…dcf8d`, and freshness is `match`, so the graph agrees with the source inputs the tool could check. There were no errors or diagnostics.

**Symbol:** searching for `leaf` among methods returned exactly one match, so there was nothing to choose between:
- `leaf(): number`, a function in `entry.ts:1`, in project `project`, built from `tsconfig.json`.

**Callers:** I asked for incoming calls one level deep, with the tool's default filters and limits of 100 nodes and 300 edges.

| Caller | Location | Signature | Relationship | Certainty | Source |
|---|---|---|---|---|---|
| `start` | `entry.ts:2` | `(flag: boolean): number` | call | direct | TypeScript compiler |

- That is the complete result: nothing was cut off or left out.
- The default filters leave out test code, generated code and unavailable targets. So this list doesn't cover callers in those categories.
- `start` takes a `flag: boolean` parameter, so the call to `leaf` may depend on that flag. This is a call found in the source code, not proof that `leaf` runs every time `start` does. I'd need to check `start`'s flow to confirm that, and you didn't ask for it.

As you asked, I didn't refresh the graph or edit any files.

### Turn 2: Same-session repeat

```json
[
  {
    "tool": "graph_status",
    "input": {
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 6,
        "edges": 1,
        "diagnostics": [],
        "diagnostics_total": 0
      }
    }
  }
]
```

Observed response:

The result hasn't changed since my previous answer: the graph is still generation `1c053c1829d1…dcf8d` and still matches the source.

I checked the graph status again and nothing differs. It's the same `codecallnavigator` server and project with the `observed-inputs=/` mapping, and there are no errors or diagnostics. You asked for the same callers of the same `leaf` symbol (`entry.ts:1`), looking one level up, with the default filters and the 100-node / 300-edge limits. So I didn't run the caller query again. My previous caller summary still applies.

I didn't refresh the graph or edit any files.

### Turn 3: Unavailable body flow

```json
[
  {
    "tool": "graph_status",
    "input": {
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 6,
        "edges": 1,
        "diagnostics": [],
        "diagnostics_total": 0
      }
    }
  },
  {
    "tool": "find_symbols",
    "input": {
      "text": "run",
      "category": "methods",
      "offset": 0,
      "limit": 20,
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "hits": [
          {
            "name": "run",
            "detail": "(): void · project · <isolated-fixture>/project/tsconfig.json",
            "target": {
              "kind": "method",
              "symbol": "S3"
            }
          }
        ],
        "offset": 0,
        "limit": 20
      }
    }
  },
  {
    "tool": "function_flow",
    "input": {
      "symbol": "S3",
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": true,
    "result_excerpt": "CLI error flow_unavailable: No source body is available for this declaration."
  }
]
```

Observed response:

I couldn't get a function flow for `Contract.run`. The tool returned an error: **`flow_unavailable`: "No source body is available for this declaration."**

**Setup:** same `codecallnavigator` server and project, with the `observed-inputs=/` mapping. I checked the graph status first: it's still generation `1c053c1829d1…dcf8d` and still matches the source, with no diagnostics.

**Symbol:** searching for `run` among methods returned exactly one match:
- `Contract.run(): void` in `entry.ts`, project `project`, built from `tsconfig.json`. The graph records it as a method declared in an interface called `Contract`, not a function with code.

**Why there's no flow:** a method declared in a TypeScript interface has no code of its own to trace, only its signature. The tool says no body is available. I have no flow to show, so I won't describe one.

**Next step:** if you want to see what actually runs when `Contract.run()` is called, I can look for classes that implement `Contract` and places that call `run`. Those are separate queries, so I'll only run them if you ask.

I didn't refresh the graph or edit any files.

### Turn 4: Stale after external edit

```json
[
  {
    "tool": "graph_status",
    "input": {
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "stale",
        "omitted_reasons": 0,
        "reasons": [
          {
            "code": "input_changed",
            "path": "@repo/other.ts"
          }
        ],
        "reasons_total": 1,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 6,
        "edges": 1,
        "diagnostics": [],
        "diagnostics_total": 0
      }
    }
  },
  {
    "tool": "graph_section",
    "input": {
      "selection": {
        "kind": "neighborhood",
        "symbols": [
          "S1"
        ],
        "direction": "Incoming",
        "depth": 2
      },
      "limits": {
        "nodes": 100,
        "edges": 300
      },
      "expected_generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
      "freshness": {
        "contract_valid": true,
        "digest": "1c053c1829d1b70058a5f5d5b849ec903c5231649bdc7da3eda9243d9a9dcf8d",
        "input_status": "stale",
        "omitted_reasons": 0,
        "reasons": [
          {
            "code": "input_changed",
            "path": "@repo/other.ts"
          }
        ],
        "reasons_total": 1,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 2,
        "edges": 1,
        "truncated": false,
        "omitted_nodes": 0,
        "omitted_edges": 0,
        "diagnostics": [],
        "diagnostics_total": 0,
        "roots": [
          "S1"
        ],
        "relationships": [
          {
            "from": "S2",
            "to": "S1",
            "kind": "call",
            "certainty": "direct",
            "provenance": "typescript.compiler",
            "classification": {
              "kind": "call",
              "resolution": "direct"
            }
          }
        ]
      }
    }
  }
]
```

Observed response:

The graph is out of date: `other.ts` has changed since it was built. Going by the graph as it was before your edit, the only code affected by changing `leaf` is `start` in `entry.ts`. The current source shows no new callers of `leaf`, but only a refresh can confirm that.

**Graph status:** same server, project and `observed-inputs=/` mapping, still generation `1c053c1829d1…dcf8d`. Freshness is now `stale`, with one reason: `input_changed` for `other.ts`. There are no diagnostics. Because the graph no longer matches the source, my earlier caller summary can't be treated as current.

**1. From the graph, as it was before your edit**

I looked two levels up from `leaf` this time, so I'd also catch whatever calls `start`. Filters were the defaults and limits were 100 nodes and 300 edges.

| Level | Caller | Location | Relationship | Certainty | Source |
|---|---|---|---|---|---|
| 1 | `start(flag: boolean): number` | `entry.ts:2` | call | direct | TypeScript compiler |
| 2 | — | — | — | — | — |

- Nothing in the graph calls `start`, so the impact stops there.
- The result is complete: nothing was cut off or left out.
- The default filters leave out test code, generated code and unavailable targets.

**2. From the current source (separate from the graph)**

- A search for `leaf` finds it only in `entry.ts:1`, where it's defined, and `entry.ts:2`, where `start` calls it inside `if(flag)`. Changing what `leaf` returns therefore changes what `start(true)` returns. `start(false)` still returns `0`.
- `other.ts` now contains only `export function start(): number { return 45; }`. That is a different function that happens to share the name `start` with the one in `entry.ts`. It doesn't import or call `leaf`.
- A text search can miss indirect references, such as `leaf` being passed around as a value or re-exported under another name. Treat this part as a rough check.

**What isn't known:** the stale graph can't tell me how your edit to `other.ts` changed its symbols or calls. For example, I can't tell whether it used to reference `leaf` or call `entry.ts`'s `start`. To get impact results that match the current code, I can run `discover_builds` and then `refresh_graph`, but only if you approve.

I didn't refresh the graph or edit any files.

### Turn 5: Explicit refresh and new generation

```json
[
  {
    "tool": "refresh_graph",
    "input": {},
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
      "freshness": null,
      "error": null,
      "result": {
        "event": "complete",
        "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
      }
    }
  },
  {
    "tool": "graph_status",
    "input": {
      "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
      "freshness": {
        "contract_valid": true,
        "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 6,
        "edges": 1,
        "diagnostics": [],
        "diagnostics_total": 0
      }
    }
  },
  {
    "tool": "find_symbols",
    "input": {
      "text": "leaf",
      "category": "methods",
      "offset": 0,
      "limit": 20,
      "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
      "freshness": {
        "contract_valid": true,
        "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "hits": [
          {
            "name": "leaf",
            "detail": "(): number · project · <isolated-fixture>/project/tsconfig.json",
            "target": {
              "kind": "method",
              "symbol": "S1"
            }
          }
        ],
        "offset": 0,
        "limit": 20
      }
    }
  },
  {
    "tool": "graph_section",
    "input": {
      "selection": {
        "kind": "neighborhood",
        "symbols": [
          "S1"
        ],
        "direction": "Incoming",
        "depth": 1
      },
      "limits": {
        "nodes": 100,
        "edges": 300
      },
      "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
    },
    "is_error": false,
    "result_excerpt": {
      "version": 1,
      "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
      "freshness": {
        "contract_valid": true,
        "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "input_status": "match",
        "omitted_reasons": 0,
        "reasons": [],
        "reasons_total": 0,
        "version": 1
      },
      "error": null,
      "result": {
        "nodes": 2,
        "edges": 1,
        "truncated": false,
        "omitted_nodes": 0,
        "omitted_edges": 0,
        "diagnostics": [],
        "diagnostics_total": 0,
        "roots": [
          "S1"
        ],
        "relationships": [
          {
            "from": "S2",
            "to": "S1",
            "kind": "call",
            "certainty": "direct",
            "provenance": "typescript.compiler",
            "classification": {
              "kind": "call",
              "resolution": "direct"
            }
          }
        ]
      }
    }
  }
]
```

Observed response:

The graph refresh finished, and the result is the same as before: `start` in `entry.ts` is still the only function that calls `leaf`. The refreshed graph matches the current source, including your edit to `other.ts`.

**Refresh:** I called `refresh_graph` once, with no build targets, so the tool picked which builds to analyse itself. It completed with no errors. That's the only change I made; I didn't touch any source files.
- **New generation:** `3df36d989027…f301059`. The old generation `1c053c18…` and the symbol IDs from it no longer apply, so I looked `leaf` up again.
- **Status after refresh:** matches the source, no diagnostics. The graph has 6 symbols and 1 relationship.
- **Roots:** `observed-inputs` = `/` and `repo` = the project directory.
- **Note:** the refresh used a TypeScript install from another project, `<typescript-node-modules>/` (TypeScript 6.0.3, Node 24.15.0). That probably comes from the `observed-inputs=/` mapping. It doesn't affect this answer, but this fixture graph does depend on that project.

**Symbol:** searching for `leaf` again returned exactly one match: `leaf(): number` at `entry.ts:1`, project `project`, built from `tsconfig.json`.

**Callers:** one level up, default filters, limits of 100 nodes and 300 edges.

| Caller | Location | Signature | Relationship | Certainty | Source |
|---|---|---|---|---|---|
| `start` | `entry.ts:2` | `(flag: boolean): number` | call | direct | TypeScript compiler |

- That's the complete result: nothing was cut off or left out.
- The default filters leave out test code, generated code and unavailable targets.
- Your edit to `other.ts` added no callers of `leaf`. Its new `start(): number` is a separate function that only shares the name `start` and doesn't call `leaf`.
- This is a call found in the source, not proof it runs: `start` only calls `leaf` when its `flag` argument is true.

## Missing graph

```json
[
  {
    "calls": [
      {
        "tool": "graph_status",
        "input": {},
        "is_error": true,
        "result_excerpt": "CLI error graph_missing: No such file or directory (os error 2)"
      }
    ],
    "summary": "I can't name any callers of `leaf`. There's no analyzed graph for this project and no source code to search.\n\n**Graph check:** I used the `codecallnavigator` MCP server, which you said is bound to this empty project with no linked roots. `graph_status` returned an error: **`graph_missing`** (\"No such file or directory (os error 2)\"). That means there's no saved graph, so there's no generation, freshness or input status. This is a failure, not an empty graph that happens to have no callers. Because of it I didn't run `find_symbols` for `leaf` or an incoming `graph_section` query.\n\n**Source check:** The project directory (`.../ccnavigator-claude-native-_8u025ez/missing`) is completely empty, with no files at all. There's no code that defines or calls `leaf`.\n\n**What would change this:**\n- If `leaf` is in a codebase somewhere else, that code needs to be linked to this project, or you need to point me at a server bound to it.\n- After that, an analysis would have to run (`discover_builds`, then an explicit `refresh_graph`). You told me not to refresh, so I didn't start one. Say if you want it once there's source to analyze.",
    "summary_bytes": 1136
  }
]
```

## Isolated lifecycle and removal

Installed package skill SHA-256: `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5`.

In temporary `CLAUDE_CONFIG_DIR`, commands registered a project-local server, disabled and uninstalled `codecallnavigator@codecallnavigator-local`, removed the local marketplace and server registration, and listed plugins. A second isolated install exercised disable/list (`enabled: false`), enable/list (`enabled: true`), uninstall/catalog removal/list again. The final registry is empty. Claude retained its downloaded plugin cache, as recorded; cache deletion is not claimed. Source files and the published graph hashes were identical before and after removal. No production configuration or engine installation was changed.

Preservation hashes:

```json
{
  "before": {
    "entry.ts": "2f694515447bb802f293a4c441d2efbf5383ef6ec78fbfe050064ed6a44f81bd",
    "other.ts": "5b0f219c58ac8f4da1580496fcd965f712119c8aac3b9ef6655e6aaf44e72036",
    "tsconfig.json": "fb549b2dad161796f36d60d0bfe4f7e3d8bd8b1b75412d9a2ee399f287e4ce1e",
    "graph.json": "9e3e9b1c7d9cffb0761d0c819a8f29c787dd30512f5ce14c167a9f8247589ca7"
  },
  "after": {
    "entry.ts": "2f694515447bb802f293a4c441d2efbf5383ef6ec78fbfe050064ed6a44f81bd",
    "other.ts": "5b0f219c58ac8f4da1580496fcd965f712119c8aac3b9ef6655e6aaf44e72036",
    "tsconfig.json": "fb549b2dad161796f36d60d0bfe4f7e3d8bd8b1b75412d9a2ee399f287e4ce1e",
    "graph.json": "9e3e9b1c7d9cffb0761d0c819a8f29c787dd30512f5ce14c167a9f8247589ca7"
  }
}
```

New session after removal:

Neither is available in this session:

- **codecallnavigator plugin skill:** Not available. It isn't in this session's list of skills.
- **CodeCallNavigator MCP tools:** Not available. A tool search for "codecallnavigator" found nothing, and no MCP tools of that kind are loaded.

As you asked, I didn't install anything, run any analysis, or look at any configuration.

Lifecycle command receipts (all run against the temporary config/project):

```json
[
  {
    "command": "claude mcp add --scope local codecallnavigator -- codecallnavigator-mcp --engine ccnavigator --project <isolated-fixture>/project",
    "exit": 0
  },
  {
    "command": "claude plugin disable codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin uninstall codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin marketplace remove codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude mcp remove --scope local codecallnavigator",
    "exit": 0
  },
  {
    "command": "claude plugin list --json",
    "exit": 0
  },
  {
    "command": "claude plugin marketplace add <repository>/target/agent-integrations/mcp-packages-final/codecallnavigator-claude-code",
    "exit": 0
  },
  {
    "command": "claude plugin install codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin disable codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin list --json",
    "exit": 0
  },
  {
    "command": "claude plugin enable codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin list --json",
    "exit": 0
  },
  {
    "command": "claude plugin uninstall codecallnavigator@codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin marketplace remove codecallnavigator-local",
    "exit": 0
  },
  {
    "command": "claude plugin list --json",
    "exit": 0
  }
]
```

The first removal was followed by the recorded new-session absence check. The subsequent isolated reinstall verified disable/enable registry states, then removed the same plugin/catalog again; the final registry was again empty and the preservation hashes stayed identical.

## Additional final-package boundary journeys

These two isolated sessions used the same retained installed cached package, still byte-identical to current source. No package/registration/source changes were required and no analysis was invoked. The server remained available through the SDK in both sessions. Native inputs below consistently abbreviate stable symbols as `B1`, `B2`, etc.; generation digests and requested bounds are preserved.

### Unknown project binding

From a different, empty temporary working directory, configure the server privately to the compiler fixture, but supply no project binding in the user prompt and no host configuration files to the model. Invoke:

> /codecallnavigator:graph-context Use CodeCallNavigator MCP to explain possible callers of leaf in my current project. Do not refresh, edit files or inspect host configuration files.

Observed native graph-tool calls: **zero**. The model asked the user to confirm the server/project and linked roots before querying; it did not treat the current directory or server name as proof of binding. Normal skill/metadata discovery is separate from querying graph state.

```json
{
  "native_calls": [],
  "other_tool_names": [],
  "response_bytes": 1176,
  "response": "I haven't run any graph queries yet. There's one MCP server available, `codecallnavigator`, but nothing I'm allowed to check shows that it's bound to your current project (`/private/var/folders/.../ccnavigator-unbound-4j_4_eh_`).\n\nThe tools can't confirm this either. A server's name, its graph generation and matching source file names don't show which project root it was set up for, and `graph_status` doesn't report that root. The only place the binding is recorded is the host configuration, and you asked me not to inspect it. If the server is bound to a different project, querying it for `leaf` could return callers from that project and present them as yours.\n\nTo go ahead, please confirm one of these:\n\n1. **The `codecallnavigator` server is bound to this project.** Also list any linked roots. I'll then run `graph_status`, find `leaf` with `find_symbols`, and list its callers one level up with `graph_section` (incoming). I won't refresh the graph or edit anything.\n2. **It isn't bound to this project, or you're not sure.** I can search the source directly instead (skipping `.codecallnavigator/`). That gives text matches for `leaf`, not graph-verified callers."
}
```

### Explicit selection and real truncation

Supply the known fixture binding and explicit test-only `observed-inputs=/` root. Invoke:

> /codecallnavigator:graph-context The configured codecallnavigator MCP server is bound to this project with the explicit test-only linked root observed-inputs=/. Explain possible callees of entry.ts start. Use symbol_details to confirm the selected method belongs to entry.ts, then request an Outgoing neighborhood at depth 1 with limits nodes=1 and edges=1. Do not refresh or edit files.

Observed `graph_status` first, two same-named search candidates, `symbol_details` confirming the selected `entry.ts` method, and generation-bound `graph_section`. The actual result returned one root node, zero edges, `truncated: true`, one omitted node and one omitted edge. The response retained those counts and declined to invent the omitted target. Its graph-context response was below 8 KiB.

```json
{
  "native_calls": [
    {
      "tool": "graph_status",
      "input": {},
      "is_error": false,
      "result_excerpt": {
        "version": 1,
        "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "freshness": {
          "contract_valid": true,
          "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
          "input_status": "match",
          "omitted_reasons": 0,
          "reasons": [],
          "reasons_total": 0,
          "version": 1
        },
        "error": null,
        "result": {
          "diagnostics": [],
          "diagnostics_total": 0,
          "edges": 1,
          "nodes": 6,
          "omitted_diagnostics": 0,
          "shortened_diagnostics": 0
        }
      }
    },
    {
      "tool": "find_symbols",
      "input": {
        "text": "start",
        "category": "methods",
        "offset": 0,
        "limit": 20,
        "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
      },
      "is_error": false,
      "result_excerpt": {
        "version": 1,
        "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "freshness": {
          "contract_valid": true,
          "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
          "input_status": "match",
          "omitted_reasons": 0,
          "reasons": [],
          "reasons_total": 0,
          "version": 1
        },
        "error": null,
        "result": {
          "hits": [
            {
              "name": "start",
              "detail": "(flag: boolean): number · project · <isolated-fixture>/project/tsconfig.json",
              "target": {
                "kind": "method",
                "symbol": "B1"
              }
            },
            {
              "name": "start",
              "detail": "(): number · project · <isolated-fixture>/project/tsconfig.json",
              "target": {
                "kind": "method",
                "symbol": "B2"
              }
            }
          ]
        }
      }
    },
    {
      "tool": "symbol_details",
      "input": {
        "symbol": "B1",
        "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
      },
      "is_error": false,
      "result_excerpt": {
        "version": 1,
        "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "freshness": {
          "contract_valid": true,
          "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
          "input_status": "match",
          "omitted_reasons": 0,
          "reasons": [],
          "reasons_total": 0,
          "version": 1
        },
        "error": null,
        "result": {
          "file": "<isolated-fixture>/project/entry.ts",
          "line": 2,
          "container": "entry.ts",
          "signature": "(flag: boolean): number",
          "project": "project",
          "build": "<isolated-fixture>/project/tsconfig.json",
          "is_test": false,
          "is_generated": false,
          "body_unavailable": false
        }
      }
    },
    {
      "tool": "graph_section",
      "input": {
        "selection": {
          "kind": "neighborhood",
          "symbols": [
            "B1"
          ],
          "direction": "Outgoing",
          "depth": 1
        },
        "limits": {
          "nodes": 1,
          "edges": 1
        },
        "expected_generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059"
      },
      "is_error": false,
      "result_excerpt": {
        "version": 1,
        "generation": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
        "freshness": {
          "contract_valid": true,
          "digest": "3df36d989027214beb8b3c927bbb3837543ad1af5324be88e288d3c2bf301059",
          "input_status": "match",
          "omitted_reasons": 0,
          "reasons": [],
          "reasons_total": 0,
          "version": 1
        },
        "error": null,
        "result": {
          "truncated": true,
          "omitted_nodes": 1,
          "omitted_edges": 1,
          "diagnostics": [],
          "diagnostics_total": 0,
          "nodes_returned": 1,
          "edges_returned": 0
        }
      }
    }
  ],
  "other_tool_names": [
    "ToolSearch"
  ],
  "response_bytes": 1465,
  "response": "The `nodes=1, edges=1` limits cut out the callee, so this query doesn't show what `entry.ts` `start` calls.\n\n**Binding and freshness:** I used the codecallnavigator server you named, bound to this project with the linked root `observed-inputs=/`. Graph status: generation `3df36d98…f301059`, input_status **match**, no diagnostics. The saved graph has 6 nodes and 1 edge in total.\n\n**Symbol selection:** Searching for `start` found two methods:\n- `entry.ts`: `start(flag: boolean): number`, the one I selected.\n- `other.ts`: `start(): number`, not selected.\n\n`symbol_details` confirmed the selected symbol is in file `…/project/entry.ts`, line 2. It is a TypeScript function in project `project`, built from `tsconfig.json`, and is not a test or generated code.\n\n**Outgoing neighborhood, depth 1, with your limits and the CLI's default filters:**\n- **Nodes returned:** only the root, `start` itself.\n- **Edges returned:** none.\n- **Omitted:** 1 node and 1 edge, and the result is flagged `truncated: true`.\n\nSo `start` has one possible outgoing relationship (call or reference) to one other symbol, but the limits left out both the target and the type of relationship. I won't guess the target, even though there's a `start` in `other.ts` that might seem likely.\n\nTo see the callee, I can rerun the same query with the default limits (`nodes=100, edges=300`) at the same generation. That's still a read-only query, with no refresh and no edits. Should I run it?"
}
```

Raw transient traces: `target/agent-integrations/claude-mcp-binding-unknown.jsonl` and `claude-mcp-bounded-final.jsonl`.

## Boundaries

These are measured conversations, not a guarantee that every future model output will follow the skill. The fixture verifies macOS local Claude Code with the existing authorized account. Other operating systems, claude.ai/Cowork, desktop UI routing and hook behavior are not claimed. The packages contain no hooks. Default-root unverifiable freshness and disabled-MCP fallback are recorded in the initial native host probes; no fabricated freshness was substituted for this match test.
