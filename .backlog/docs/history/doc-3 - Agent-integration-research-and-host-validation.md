---
id: doc-3
title: Agent integration research and host validation
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:16:00Z'
---
# Agent integration research and host validation


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-agent-integrations-readme-md"></a>

## evidence/agent-integrations/README.md

Source: [evidence/agent-integrations/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/README.md). Historical record; retained rationale, methods and limitations.

### Agent integration research evidence

2026-11-12, macOS arm64. The TASK-51 research below is historical. Current
TASK-52/53/54 delivery uses native MCP tools in skill-only host packages.

- [Current native MCP transcript](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/native-mcp-protocol.json): 19 real compiler-backed
  tool operations, complete envelopes and measured response sizes/timings.
  Dependency paths are normalized to `<typescript-node-modules>`; no dependency
  project source was analyzed.
- [Delivery evidence](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-delivery-mcp-md>): current package and host acceptance.

#### Historical TASK-51 research

Scope: research; no shipped plugin or runtime code changes in that spike.

- [Protocol transcript](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-protocol-md>): 16 MCP tool operations plus unavailable-engine startup failure, real Node 24.15.0 / TypeScript 6.0.3 compiler fixture. Complete responses, timing and serialized response byte counts retained. Temporary project addresses normalized; no source from the dependency project analyzed.
- [Host probes](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-hosts-md>): codex-cli 0.166.0 and Claude Code 2.1.306 isolated configuration operations and Claude minimal plugin validation, all exit zero. No model sessions started, production plugins installed, or global hooks changed.
- [Report](<../../../docs/agent-integrations.md>): official source matrix, journeys, defaults and follow-up design.

The initial probe assertion incorrectly treated a present null error as failure; corrected to check its value. The recorded final run passed. Missing graph did not create an artifact; two same-named methods stayed ambiguous; a 1-node selection reported truncation; a body-less interface method reported unavailable flow; editing changed freshness to stale; read queries left artifact bytes identical; explicit refresh changed generation and rejected the old generation. Missing CLI startup exited nonzero. MCP process closed/reaped after each run. Initial freshness was unverifiable (unmapped_input), not fresh; the report requires preserving that distinction. No large-project benchmark claim.

#### Reproduce

Run from the repository root with the separately packaged binaries present in dist/CodeCallNavigatorEngine/bin and dist/CodeCallNavigatorMcp/bin. Save each block below as a temporary Python file, replace the dependency directory in the protocol probe with a TypeScript 6.0.x node_modules installation, then run with Python 3.13 and Node 24+ on PATH. The scripts only create temporary fixture/configuration directories and rewrite these evidence Markdown files. They are research recipes, not installed automation or production test additions.

Host interactive skill invocation, full plugin lifecycle, disabled/untrusted hooks and cross-platform behavior were not tested. Registration and manifest validation must not be read as proof that a model selected or followed the proposed skill. Follow-up acceptance must cover those cases without bypassing normal host trust.

##### Protocol probe

```python
import sys,json,subprocess,tempfile,pathlib,queue,threading,time,os
repo=pathlib.Path.cwd(); sys.path.insert(0,str(repo/'tools')); import mcp_smoke as m
engine=repo/'dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine'; binary=repo/'dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp'
records=[]
with tempfile.TemporaryDirectory(prefix='agent-graph-') as tmp:
 root=pathlib.Path(tmp); (root/'node_modules').symlink_to('/path/to/node_modules')
 (root/'tsconfig.json').write_text('{"compilerOptions":{"strict":true,"target":"ES2022","types":[]},"include":["*.ts"]}')
 (root/'entry.ts').write_text('export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\nexport interface Contract { run(): void; }\n')
 (root/'other.ts').write_text('export function start(): number { return 2; }\n')
 env=os.environ.copy(); env['CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT']='/path/to/node_modules/typescript'
 p=subprocess.Popen([str(binary),'--engine',str(engine),'--project',str(root)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env)
 q=queue.Queue(); threading.Thread(target=m.output_lines,args=(p.stdout,q),daemon=True).start(); seq=0
 def req(method,params):
  global seq
  seq+=1; m.send(p,method,seq,params); return m.response(q,seq)
 def call(name,args):
  t=time.monotonic(); r=req('tools/call',dict(name=name,arguments=args)); records.append(dict(tool=name,arguments=args,milliseconds=round((time.monotonic()-t)*1000,2),bytes=len(json.dumps(r).encode()),response=r)); return r.get('structuredContent',{})
 try:
  req('initialize',dict(protocolVersion='2025-11-25',capabilities={},clientInfo=dict(name='integration-research',version='1'))); m.send(p,'notifications/initialized')
  assert call('graph_status',{})['error']['code']=='graph_missing'; assert not (root/'.codecallnavigator/graph.json').exists()
  call('discover_builds',{}); refreshed=call('refresh_graph',{}); assert not refreshed.get('error'),refreshed
  g=call('graph_status',{})['generation']; hits=call('find_symbols',dict(text='start',category='methods',expected_generation=g))['result']['hits']; assert len(hits)==2
  symbol=hits[0]['target']['symbol']; details=call('symbol_details',dict(symbol=symbol,expected_generation=g))
  call('function_flow',dict(symbol=symbol,expected_generation=g))
  call('neighbors',dict(symbol=symbol,limits=dict(nodes=1,edges=1),expected_generation=g))
  section=call('graph_section',dict(selection=dict(kind='neighborhood',symbols=[symbol],direction='Both',depth=2),limits=dict(nodes=1,edges=1),expected_generation=g)); assert section['result']['truncated']
  contracts=call('find_symbols',dict(text='run',category='methods',expected_generation=g))['result']['hits']
  assert contracts, 'Expected interface method'; call('function_flow',dict(symbol=contracts[0]['target']['symbol'],expected_generation=g))
  before=(root/'.codecallnavigator/graph.json').read_bytes(); (root/'other.ts').write_text('export function start(): number { return 42; }\n')
  stale=call('graph_status',{}); assert stale['freshness']['input_status']=='stale'
  call('symbol_source',dict(symbol=symbol,expected_generation=g)); call('graph_status',dict(expected_generation=g)); assert (root/'.codecallnavigator/graph.json').read_bytes()==before
  call('refresh_graph',{}); assert call('graph_status',dict(expected_generation=g))['error']['code']=='generation_mismatch'
 finally:
  p.stdin.close(); p.wait(timeout=20)
 failed=subprocess.run([str(binary),'--engine',str(root/'absent-engine'),'--project',str(root)],capture_output=True,text=True,timeout=20)
 assert failed.returncode!=0
 records.append(dict(scenario='unavailable-engine',exit=failed.returncode,stderr=failed.stderr))
 rendered=json.dumps(records,indent=2).replace(str(root),'<temporary-project>').replace(str(repo),'<repository>')
 (repo/'evidence/agent-integrations/protocol.md').write_text('# Real compiler MCP probe\n\n2026-11-12; Node 24.15.0, TypeScript 6.0.3; packaged engine and MCP, JSON-line stdio.\n\n```json\n'+rendered+'\n```\n')
 print('PASS',len(records),'recorded operations')

```

##### Host configuration probe

```python
import subprocess,tempfile,pathlib,os,json
repo=pathlib.Path.cwd(); output=[]
with tempfile.TemporaryDirectory(prefix='agent-host-') as tmp:
 base=pathlib.Path(tmp); env=os.environ.copy(); env['CODEX_HOME']=str(base/'codex'); env['CLAUDE_CONFIG_DIR']=str(base/'claude'); (base/'codex').mkdir(); (base/'claude').mkdir()
 binary=str(repo/'dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp'); engine=str(repo/'dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine')
 commands=[['codex','mcp','add','graph-research','--',binary,'--engine',engine,'--project',str(base)],['codex','mcp','get','graph-research'],['codex','mcp','remove','graph-research'],['codex','mcp','list'],['claude','mcp','add','--scope','user','graph-research','--',binary,'--engine',engine,'--project',str(base)],['claude','mcp','remove','--scope','user','graph-research']]
 plugin=base/'plugin'; (plugin/'.claude-plugin').mkdir(parents=True); (plugin/'skills/graph-context').mkdir(parents=True)
 (plugin/'.claude-plugin/plugin.json').write_text(json.dumps(dict(name='graph-research',version='0.0.1',description='Temporary manifest feasibility probe')))
 (plugin/'skills/graph-context/SKILL.md').write_text('---\nname: graph-context\ndescription: Query graph context on explicit request.\n---\nUse graph_status before find_symbols. Never refresh implicitly.\n')
 commands.append(['claude','plugin','validate',str(plugin)])
 for c in commands:
  r=subprocess.run(c,cwd=base,env=env,capture_output=True,text=True,timeout=25); output.append(dict(command=c,exit=r.returncode,stdout=r.stdout,stderr=r.stderr))
 rendered=json.dumps(output,indent=2).replace(str(base),'<isolated-config>').replace(str(repo),'<repository>')
 (repo/'evidence/agent-integrations/hosts.md').write_text('# Isolated host configuration probes\n\n2026-11-12; codex-cli 0.166.0; Claude Code 2.1.306. Temporary CODEX_HOME and CLAUDE_CONFIG_DIR; no production configuration changed. These are configuration/manifest checks, not model invocation or interactive hook-trust tests.\n\n```json\n'+rendered+'\n```\n')
 print([(x['command'][:3],x['exit']) for x in output])

```


<a id="source-evidence-agent-integrations-claude-native-mcp-md"></a>

## evidence/agent-integrations/claude-native-mcp.md

Source: [evidence/agent-integrations/claude-native-mcp.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md). Historical record; retained rationale, methods and limitations.

### Claude Code native MCP acceptance

2026-11-12, macOS arm64, Claude Code 2.1.306. Actual model sessions used the installed plugin's cached skill and the separately installed Rust SDK MCP server over stdio. No shell MCP shim or direct CLI query wrapper was used by the model. Engine commands are resolved through PATH: `codecallnavigator-mcp --engine ccnavigator --project PROJECT`.

This document retains sanitized native tool inputs, selected result fields, output excerpts and byte counts. Tool-result excerpts are deliberately reduced evidence, not a replacement MCP wire contract; exact full tool envelopes were checked independently by `tools/agent_mcp_journeys.py`. Stable symbol strings are consistently shortened to `S1`, `S2`, etc. Temporary fixture/install and dependency locations are placeholders. No authentication data, hidden reasoning or message signatures are retained here.

#### Reproduction recipe

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

#### Observed results

| Turn | Native graph tools | Output bytes | Outcome |
| --- | --- | ---: | --- |
| Real match callers | graph_status, find_symbols, graph_section | 1429 | Recorded below |
| Same-session repeat | graph_status | 605 | No repeated section |
| Unavailable body flow | graph_status, find_symbols, function_flow | 1125 | Recorded below |
| Stale after external edit | graph_status, graph_section | 2278 | Recorded below |
| Explicit refresh and new generation | refresh_graph, graph_status, find_symbols, graph_section | 1928 | Recorded below |

Every analyzed query binds `expected_generation`; ordinary turns do not invoke refresh. The repeat turn performs exactly one native `graph_status`, keeps the same real `match` generation, and refers to the prior caller summary without a graph-section query or repeated caller content. Stale impact is labeled as earlier analyzed evidence and kept separate from current-source observations. Only the explicitly authorized fifth turn calls `refresh_graph`, publishes a new generation, requests status, resolves the method again, then queries the new generation.

Claude exposes successful native tool results as full JSON envelopes. For `isError` tool results it renders the server's text block (for example `CLI error flow_unavailable: No source body is available for this declaration.`) with `is_error: true`; this host transcript does not expose structured error generation/freshness metadata. The model retained the error code/message and used the preceding status check for graph context instead of inventing a flow.

##### Turn 1: Real match callers


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


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

##### Turn 2: Same-session repeat

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

##### Turn 3: Unavailable body flow


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


Observed response:

I couldn't get a function flow for `Contract.run`. The tool returned an error: **`flow_unavailable`: "No source body is available for this declaration."**

**Setup:** same `codecallnavigator` server and project, with the `observed-inputs=/` mapping. I checked the graph status first: it's still generation `1c053c1829d1…dcf8d` and still matches the source, with no diagnostics.

**Symbol:** searching for `run` among methods returned exactly one match:
- `Contract.run(): void` in `entry.ts`, project `project`, built from `tsconfig.json`. The graph records it as a method declared in an interface called `Contract`, not a function with code.

**Why there's no flow:** a method declared in a TypeScript interface has no code of its own to trace, only its signature. The tool says no body is available. I have no flow to show, so I won't describe one.

**Next step:** if you want to see what actually runs when `Contract.run()` is called, I can look for classes that implement `Contract` and places that call `run`. Those are separate queries, so I'll only run them if you ask.

I didn't refresh the graph or edit any files.

##### Turn 4: Stale after external edit


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


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

##### Turn 5: Explicit refresh and new generation


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


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

#### Missing graph


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


#### Isolated lifecycle and removal

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


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


The first removal was followed by the recorded new-session absence check. The subsequent isolated reinstall verified disable/enable registry states, then removed the same plugin/catalog again; the final registry was again empty and the preservation hashes stayed identical.

#### Additional final-package boundary journeys

These two isolated sessions used the same retained installed cached package, still byte-identical to current source. No package/registration/source changes were required and no analysis was invoked. The server remained available through the SDK in both sessions. Native inputs below consistently abbreviate stable symbols as `B1`, `B2`, etc.; generation digests and requested bounds are preserved.

##### Unknown project binding

From a different, empty temporary working directory, configure the server privately to the compiler fixture, but supply no project binding in the user prompt and no host configuration files to the model. Invoke:

> /codecallnavigator:graph-context Use CodeCallNavigator MCP to explain possible callers of leaf in my current project. Do not refresh, edit files or inspect host configuration files.

Observed native graph-tool calls: **zero**. The model asked the user to confirm the server/project and linked roots before querying; it did not treat the current directory or server name as proof of binding. Normal skill/metadata discovery is separate from querying graph state.


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


##### Explicit selection and real truncation

Supply the known fixture binding and explicit test-only `observed-inputs=/` root. Invoke:

> /codecallnavigator:graph-context The configured codecallnavigator MCP server is bound to this project with the explicit test-only linked root observed-inputs=/. Explain possible callees of entry.ts start. Use symbol_details to confirm the selected method belongs to entry.ts, then request an Outgoing neighborhood at depth 1 with limits nodes=1 and edges=1. Do not refresh or edit files.

Observed `graph_status` first, two same-named search candidates, `symbol_details` confirming the selected `entry.ts` method, and generation-bound `graph_section`. The actual result returned one root node, zero edges, `truncated: true`, one omitted node and one omitted edge. The response retained those counts and declined to invent the omitted target. Its graph-context response was below 8 KiB.


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/claude-native-mcp.md).


Raw transient traces: `target/agent-integrations/claude-mcp-binding-unknown.jsonl` and `claude-mcp-bounded-final.jsonl`.

#### Boundaries

These are measured conversations, not a guarantee that every future model output will follow the skill. The fixture verifies macOS local Claude Code with the existing authorized account. Other operating systems, claude.ai/Cowork, desktop UI routing and hook behavior are not claimed. The packages contain no hooks. Default-root unverifiable freshness and disabled-MCP fallback are recorded in the initial native host probes; no fabricated freshness was substituted for this match test.


<a id="source-evidence-agent-integrations-codex-native-mcp-md"></a>

## evidence/agent-integrations/codex-native-mcp.md

Source: [evidence/agent-integrations/codex-native-mcp.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md). Historical record; retained rationale, methods and limitations.

### Codex package: native MCP acceptance

Date: 2026-11-12. Host: codex-cli 0.166.0 on macOS arm64, normal authenticated
host execution with read-only terminal sandbox. No approval/sandbox/hook-trust
bypass was used. This evidence exercises the final installed MCP skill package,
not the earlier direct-CLI package route. Model selection used the host default.

The plugin was built at `target/agent-integrations/mcp-packages-final/codecallnavigator-codex`,
registered as a local marketplace, installed into an isolated CODEX_HOME, and
its installed SKILL.md bytes were checked against the artifact. The separately
installed real SDK MCP server invoked `ccnavigator` by command name on PATH.
Temporary config and test project only; production host configuration was not
changed. Existing authenticated access was referenced without copying its
contents into this evidence. Other managed/cloud plugins coexisted and were
preserved.

#### Reproduction

Build the package to a new directory with the repository packager. Use a temporary
host configuration and a compiler fixture generated by `agent_journeys.fixture`
(TypeScript 6.0.3, Node 24.15.0). Put the separately packaged engine/MCP `bin`
directories on PATH. Authenticate the isolated host normally; never serialize
credentials into reports. Register/install the extracted package:

```sh
codex plugin marketplace add /path/to/extracted/codecallnavigator-codex
codex plugin add codecallnavigator@codecallnavigator-local
```

In the isolated config, use the actual fixture root as the data address:

```toml
[mcp_servers.codecallnavigator]
command = "codecallnavigator-mcp"
args = ["--engine", "ccnavigator", "--project", "/path/to/fixture"]
tool_timeout_sec = 1800
```

Use normal trusted-project/terminal settings. Seed the fixture through an
explicit acceptance-setup analysis; this setup command is not a plugin fallback.
Run new conversations and resume the returned thread for repeat/flow/edit
journeys, passing the prompt as one quoted argument:

```sh
codex exec --json --skip-git-repo-check -s read-only -C /path/to/fixture "$PROMPT"
codex exec --json --skip-git-repo-check -s read-only -C /path/to/fixture resume "$THREAD" "$PROMPT"
```

State the trusted server-to-project mapping in the opening prompt. Native tools,
not shell graph queries, perform all conversational graph operations.

The default fixture reports `unverifiable` (`unmapped_input`). To exercise real
`match`, acceptance setup reanalyzed with explicit **test-only** root mapping
`--root observed-inputs=/` and configured that same argument on the server. It
maps compiler-observed external fingerprints rather than fabricating freshness;
it is not an installed default or a recommendation to expose this root to agents.
For stale behavior the harness externally changed leaf's return from 1 to 9;
read-only conversations preserved the source/artifact hashes. Only the explicit
native refresh request replaced the graph, without changing source.

For disabled-server coverage, set `enabled = false` on this isolated server
while leaving ccnavigator on PATH and the plugin enabled. For lifecycle coverage,
disable then enable the target plugin's existing configuration entry, verify its
reported state, uninstall only the target, remove its local catalog and separately
remove the test MCP registration. Verify a new conversation has neither target
skill nor native tools. Preserve independent binaries, source and graph data.

#### Observed behavior and limits

- Explicit invocation asked which of the two start methods was intended. It
  preserved generation binding in native requests, but its final ambiguity
  prompt reported freshness **without printing the generation digest**. This is
  an observed presentation limitation; no stronger universal output claim is made.
- Implicit request selected the installed skill and used native MCP for the
  incoming leaf section at depth 1, 100 nodes/300 edges with default filters.
- Real-match same-thread repeat made only graph_status, referenced the earlier
  answer and did not repeat its caller list/section. Unverifiable repeat stayed
  qualified as earlier analyzed evidence, not verified current source.
- Body-less Contract.run returned flow_unavailable. Stale status was separated
  from live source assessment. Missing graph produced null generation/freshness
  and did not publish an artifact. No implicit refresh occurred.
- Explicitly authorized native discover_builds/refresh_graph replaced generation;
  subsequent status/search/details/selection used the new graph.
- Disabled MCP used only source fallback, with explicit .codecallnavigator
  exclusion, and made no terminal graph query despite ccnavigator remaining on
  PATH. No automatic registration/install/analysis occurred.
- Final response sizes below are measured original UTF-8 bytes, all under 8 KiB.
  This small fixture does not establish large-project context/latency behavior.
- Desktop UI, IDE/cloud/web, other operating systems, optional hooks and all
  possible host/model trajectories remain unvalidated.

#### Sanitized per-turn excerpts

These excerpts retain only completed native tool inputs, selected structured
result metadata and final user-visible output. Thinking, auth material and opaque
model signatures are not copied. Temporary paths and stable method IDs are
replaced with consistent placeholders; generation digests remain verbatim so
binding can be inspected. Whole graph payloads and source are omitted here;
raw test traces are under target/agent-integrations/task54-native-*.jsonl.

Installed skill SHA-256: `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5`.

| Journey | Native tools | Final UTF-8 bytes |
| --- | --- | --- |
| explicit | graph_status, find_symbols, find_symbols, symbol_details, symbol_details | 748 |
| implicit | graph_status, find_symbols, find_symbols, symbol_details, graph_section | 914 |
| unverifiable-repeat | graph_status | 679 |
| match-callers | graph_status, find_symbols, find_symbols, symbol_details, graph_section | 749 |
| match-repeat | graph_status | 445 |
| flow | graph_status, find_symbols, symbol_details, function_flow | 631 |
| stale | graph_status | 1014 |
| refresh | graph_status, discover_builds, refresh_graph, graph_status, find_symbols, symbol_details, graph_section | 828 |
| missing | graph_status | 423 |
| disabled | none | 517 |
| removed | none | 268 |

##### explicit


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Final user-visible response (sanitized):

```text
Which `start` do you mean?

- [entry.ts:2](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:2): `start(flag: boolean): number`
- [other.ts:1](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/other.ts:1): `start(): number`

The graph reports freshness as **unverifiable** (`unmapped_input`), so agreement with current source is unknown. Nothing was refreshed or edited.

The [graph-context skill](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/isolated-config>/plugins/cache/codecallnavigator-local/codecallnavigator/0.1.0/skills/graph-context/SKILL.md) requires asking when “multiple candidates still fit the question.”
```

##### implicit


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Final user-visible response (sanitized):

```text
The saved graph identifies one caller of `leaf(): number` at [entry.ts:1](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:1):

- `start(flag: boolean): number` at [entry.ts:2](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:2) → `leaf`. One direct call relationship, reported by `typescript.compiler`; this is static evidence, not observed execution.

The incoming query used depth 1, CLI-default filters, and limits of 100 nodes/300 edges. Nothing was omitted; no diagnostics or errors occurred.

Freshness is **unverifiable** because of `unmapped_input`, so agreement with current source is unknown. Generation: `ac70d9c79cdb0a3e356c9d87465698fe95a30fc6039922a64ac5731346728a39`. Server: `codecallnavigator`, bound to the specified project with no linked roots. No refresh or file edits were performed.
```

##### unverifiable-repeat

```json
{
  "calls": [
    {
      "server": "codecallnavigator",
      "tool": "graph_status",
      "arguments": {},
      "result": {
        "generation": "ac70d9c79cdb0a3e356c9d87465698fe95a30fc6039922a64ac5731346728a39",
        "error": null,
        "freshness": {
          "input_status": "unverifiable",
          "reasons": [
            {
              "code": "unmapped_input"
            }
          ],
          "omitted_reasons": 0
        },
        "nodes_count": 6,
        "edges_count": 1
      }
    }
  ]
}
```

Final user-visible response (sanitized):

```text
In the previously queried saved graph, `start(flag: boolean): number` at `entry.ts:2` calls `leaf(): number` at `entry.ts:1`. The relationship is classified as a **direct call**, derived by `typescript.compiler`, rather than observed runtime execution.

The generation remains `ac70d9c79cdb0a3e356c9d87465698fe95a30fc6039922a64ac5731346728a39`, but freshness is still **unverifiable** (`unmapped_input`). This repeats the earlier analyzed-state finding; it does not verify current source.

Scope is unchanged: the same project, incoming depth 1, CLI-default filters, and bounds of 100 nodes/300 edges. The earlier result had no omissions or diagnostics. No refresh was performed.
```

##### match-callers


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Final user-visible response (sanitized):

```text
`leaf(): number` at [entry.ts:1](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:1) has one caller in the graph: `start(flag: boolean): number` at line 2.

The edge `start → leaf` is a **direct call**, derived from `typescript.compiler`. This establishes a source relationship, not observed runtime execution.

Checked the configured `codecallnavigator` binding for this workspace and linked root `observed-inputs=/`. The incoming query used depth 1, CLI-default filters, and limits of 100 nodes/300 edges. Nothing was omitted; no diagnostics were reported.

Freshness: `match`. Generation: `89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e`. No refresh or file edits occurred.
```

##### match-repeat

```json
{
  "calls": [
    {
      "server": "codecallnavigator",
      "tool": "graph_status",
      "arguments": {},
      "result": {
        "generation": "89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e",
        "error": null,
        "freshness": {
          "input_status": "match",
          "reasons": [],
          "omitted_reasons": 0
        },
        "nodes_count": 6,
        "edges_count": 1
      }
    }
  ]
}
```

Final user-visible response (sanitized):

```text
Unchanged: freshness is `match` at generation `89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e`. See the previous caller summary; its source-derived relationships remain applicable.

Scope remains the configured workspace project, explicit linked root `observed-inputs=/`, `leaf`, Incoming direction, depth 1, CLI-default filters, and bounds of 100 nodes/300 edges.

Status reports no errors or diagnostics. No refresh occurred.
```

##### flow


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Final user-visible response (sanitized):

```text
`Contract.run(): void` is an interface method declaration at [entry.ts:3](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:3). It has no source body, so there is no implementation flow to explain.

The flow tool returned `flow_unavailable`: “No source body is available for this declaration.” This is unavailable flow evidence, not an empty execution path.

The same workspace project and explicit mapping `observed-inputs=/` were used. Freshness remains `match` at generation `89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e`. No refresh or edits occurred.
```

##### stale

```json
{
  "calls": [
    {
      "server": "codecallnavigator",
      "tool": "graph_status",
      "arguments": {},
      "result": {
        "generation": "89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e",
        "error": null,
        "freshness": {
          "input_status": "stale",
          "reasons": [
            {
              "code": "input_changed",
              "path": "@repo/entry.ts"
            }
          ],
          "omitted_reasons": 0
        },
        "nodes_count": 6,
        "edges_count": 1
      }
    }
  ]
}
```

Final user-visible response (sanitized):

```text
Editing `leaf` could affect `start(flag)` in [entry.ts:2](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:2). Live source shows:

- When `flag` is true, `start` returns `leaf()`—currently `9`. Changes to its return value, exceptions, or side effects can propagate through this branch.
- When `flag` is false, `start` returns `0` without calling `leaf`.
- Changing `leaf`’s parameters or return type may require updating this caller.

The workspace source search found no other references to `leaf`, and no test files were found. This does not rule out external consumers of the exported function.

**The saved graph is stale:** `input_changed` for `@repo/entry.ts`, at generation `89a06a071a769d4a0d9bf52329b75bdbc78f76825e2d88dac72e48bf4ecd2f1e`. Its earlier `start → leaf` edge is historical evidence; the assessment above comes separately from live source.

Used the same project and explicit mapping `observed-inputs=/`. No refresh or edits occurred.
```

##### refresh


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Final user-visible response (sanitized):

```text
Refresh completed successfully using native `discover_builds` and `refresh_graph`, with the discovered default build `project/tsconfig.json`. The same server/project and roots (`repo` = workspace, `observed-inputs=/`) were retained.

Resolved anew: `leaf(): number` has one caller, `start(flag: boolean): number`, at [entry.ts:2](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:2). The relationship is a **direct call**, derived from `typescript.compiler`, not observed runtime execution.

The Incoming query used depth 1, CLI-default filters, and bounds of 100 nodes/300 edges. No omissions, truncation, diagnostics, or errors occurred.

Freshness is `match` at new generation `8397b32ee9d19859b823fbf9d427b91f037f61bc925595a0b6234630c505b952`. No source files were edited.
```

##### missing

```json
{
  "calls": [
    {
      "server": "codecallnavigator",
      "tool": "graph_status",
      "arguments": {},
      "result": {
        "generation": null,
        "error": {
          "code": "graph_missing",
          "message": "No such file or directory (os error 2)"
        },
        "freshness": null
      }
    }
  ]
}
```

Final user-visible response (sanitized):

```text
The configured `codecallnavigator` server returned `graph_missing`: “No such file or directory (os error 2)” for `<missing-project>`.

Generation, freshness, and result are all null. There is no saved graph available to explain callers of `leaf`; this does **not** establish that it has no callers.

No analysis or refresh was performed.
```

##### disabled

```json
{
  "calls": []
}
```

Final user-visible response (sanitized):

```text
The CodeCallNavigator MCP server is disabled, so this is based on source inspection; graph generation and freshness are unavailable.

- `start(flag: boolean)` in entry.ts:2 directly calls `leaf()` when `flag` is true. It returns `9`; otherwise, `start` returns `0`.
- `start()` in other.ts:1 does not call `leaf`.

This is the only caller found in the workspace source. It describes a possible execution path, not observed runtime behavior. No files were refreshed or edited.
```

##### removed

```json
{
  "calls": []
}
```

Final user-visible response (sanitized):

```text
- **Codecallnavigator graph-context skill:** Not available in this session’s skill list.
- **Native codecallnavigator MCP tools:** Not available in the exposed tool list.

No files or configuration were read, and nothing was refreshed, queried, installed, or edited.
```

#### Isolated lifecycle and preservation

```json
{
  "disable_enable_verified": true,
  "target_absent": true,
  "cache_removed": true,
  "server_removed": true,
  "unrelated_plugins_preserved": true,
  "unrelated_plugin_count": 11,
  "source_graph_preserved": true,
  "before": {
    "entry.ts": "21492ab89a1cab1807ec7fbf3b791c159bda5d4dc287515a59ad687e0e266f6c",
    "other.ts": "575756e31680555bed77dbf8902360a33d625ae4cfb250711bd7a20f8e97ecb9",
    "tsconfig.json": "fb549b2dad161796f36d60d0bfe4f7e3d8bd8b1b75412d9a2ee399f287e4ce1e",
    "graph.json": "ae11cc86a85184e2eead25545019c485b334349da70033605fce00cfa4cab6bd"
  },
  "after": {
    "entry.ts": "21492ab89a1cab1807ec7fbf3b791c159bda5d4dc287515a59ad687e0e266f6c",
    "other.ts": "575756e31680555bed77dbf8902360a33d625ae4cfb250711bd7a20f8e97ecb9",
    "tsconfig.json": "fb549b2dad161796f36d60d0bfe4f7e3d8bd8b1b75412d9a2ee399f287e4ce1e",
    "graph.json": "ae11cc86a85184e2eead25545019c485b334349da70033605fce00cfa4cab6bd"
  }
}
```

Target disable/enable, uninstall/cache removal, separate MCP removal and local
marketplace removal were verified. Unrelated managed plugins remained present.
Source and graph bytes were unchanged by lifecycle operations.

#### Additional review acceptance: binding and real truncation

A second fresh temporary CODEX_HOME installed the same final artifact; the
previous lifecycle/removal proof above was not reused or changed. Its isolated
server remained bound to the existing compiler fixture. For the unknown-binding
case, Codex started in a different empty workspace and received no user-supplied
server/project mapping. It asked whether the configured different project was
intended **before any native graph call**. For actual truncation, a separate
conversation explicitly selected entry.ts start and the known project mapping,
requesting Outgoing depth 1 with exactly 1 node/1 edge. The native section really
returned truncated=true with one node and one edge omitted; the answer preserved
that limitation, generation/freshness and did not infer an absent callee.

Reproduce using the same install/config recipe in another temporary context.
Start the unknown-binding turn with `-C` pointing at an empty directory and:
`$graph-context Explain possible callers of leaf for this current workspace.
Do not refresh or edit files.` For truncation, start at the fixture and explicitly
state its trusted server/root mapping, then ask:
`$graph-context Explain possible callees of entry.ts start using Outgoing
direction, depth 1, and limits of exactly 1 node and 1 edge. Do not refresh or
edit files.` These are actual prompts, not instructions to fabricate the result.

| Additional journey | Native tools | Final UTF-8 bytes |
| --- | --- | --- |
| unknown-binding | none | 572 |
| truncated | graph_status, find_symbols, symbol_details, graph_section | 755 |

##### unknown-binding

```json
{
  "calls": []
}
```

Sanitized final user-visible response:

```text
The current workspace is empty, and the configured CodeCallNavigator server is bound to a different project: `ccnavigator-task54-qabnygkt/project`.

The [graph-context skill](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/extra-isolated-context>/config/plugins/cache/codecallnavigator-local/codecallnavigator/0.1.0/skills/graph-context/SKILL.md) requires: “If the binding is unclear, ask before querying.”

Did you intend that configured project for the `leaf` caller investigation? I haven’t queried its graph, refreshed, or edited files.
```

##### truncated


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


Sanitized final user-visible response:

```text
Resolved [`entry.ts:2`](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/project>/entry.ts:2): `start(flag: boolean): number`.

The **Outgoing**, depth **1** query used exactly **1 node / 1 edge** limits and CLI-default filters. The root consumed the node limit: only `start` was returned, with **1 node and 1 edge omitted** (`truncated: true`). Therefore, this bounded result cannot identify the possible callee; it does **not** establish that no callees exist.

Native server: `codecallnavigator`, bound to the specified workspace with `observed-inputs=/`. Freshness: `match`; generation: `8397b32ee9d19859b823fbf9d427b91f037f61bc925595a0b6234630c505b952`. No errors or diagnostics. No refresh or file edits performed.
```

Fresh-context cleanup and preservation:


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/codex-native-mcp.md).


The test target plugin, catalog and MCP registration were removed normally;
the fresh temporary configuration directory was then removed. Original source
and graph hashes remained unchanged. Raw traces: task54-native-unknown-binding.jsonl
and task54-native-truncated.jsonl under target/agent-integrations.


<a id="source-evidence-agent-integrations-delivery-mcp-md"></a>

## evidence/agent-integrations/delivery-mcp.md

Source: [evidence/agent-integrations/delivery-mcp.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/delivery-mcp.md). Historical record; retained rationale, methods and limitations.

### Native MCP companion delivery

2026-11-12; TASK-52, TASK-53 and TASK-54. This records the current native MCP
route and supersedes the direct-terminal plugin route in
[the earlier TASK-52 record](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-delivery-task52-md>). The public `ccnavigator`
terminal command remains available independently.

#### Delivered boundary

Both host packages contain the same [canonical workflow](<../../../integrations/shared/graph-context.md>),
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

#### Compiler-backed native MCP

[Complete measured envelopes](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/native-mcp-protocol.json) retain 19 real SDK stdio
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

#### Artifact identity

Canonical body SHA-256:
`49d7b3804302e3a006e156ebdc17502dda73965c09160af11375a6634008bf32`.

The isolated installation reports for both hosts measured the installed cached
skill SHA-256 as
`1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5`,
matching the generated canonical wrapper. Final isolated host tests use these
deterministic archives:

- Claude Code: `5b4d656eb9125b81dcf81550f6da904763748d86f6a8b3b74d00a416c53cd53f`
- Codex: `c29c881241eae62403616067253f77a53cfe0ce8ac5ddffd3773226834c0fce7`

#### Host evidence and limits

Host conversations and lifecycle checks are recorded separately from protocol
acceptance. The [initial Claude transcript review](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-native-mcp-hosts-initial-md>)
covers explicit/implicit native calls and disabled-MCP fallback. The extended
[Claude Code](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-claude-native-mcp-md>) and [Codex](<doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-codex-native-mcp-md>) host records
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

#### Review

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


#### Full verification

`./verify-full.sh` passed **all 33 stages** at
`target/verification/agent-integrations-mcp-full`; the
[portable stage summary](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/native-mcp-verification.json) retains statuses and
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


<a id="source-evidence-agent-integrations-delivery-task52-md"></a>

## evidence/agent-integrations/delivery-task52.md

Source: [evidence/agent-integrations/delivery-task52.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/delivery-task52.md). Historical record; retained rationale, methods and limitations.

### TASK-52 direct CLI delivery acceptance

2026-11-12; macOS arm64; codex-cli 0.166.0 and Claude Code 2.1.306.

These are real model conversations using temporary copies of the canonical skill, not evidence of finished host package installation. All graph queries used the real compiler-produced fixture through `ccnavigator` on PATH. No MCP server was configured.

#### Backend and public command

Independent tester passed 20 real CLI operations at `target/agent-integrations/task52-direct-tester.json`: capabilities, graph_missing/no publication, two ambiguous methods, directed sections, truncation, flow_unavailable, stale inputs, unchanged bytes under queries, explicit analysis/generation mismatch, and missing executable. Initial default-root freshness was unverifiable, with reasons preserved.

The PATH regression in `crates/codecallnavigator-engine/tests/public_command.rs` failed before the public command existed and passed afterward. It compares capabilities with the internal engine, checks public help and verifies the structured exit-2 missing-graph response.

#### Host preparation and limits

A separate fixture explicitly mapped `observed-inputs=/` solely to represent compiler-observed ancestor manifests and dependency inputs, allowing real freshness=match. Analysis still targeted only its temporary tsconfig; the filesystem root was not scanned. This broad test-only metadata mapping is **not** a recommended installation or skill default. No graph or freshness envelope was fabricated.

Codex used temporary CODEX_HOME with a temporary reference to the existing login, a trusted temporary project, and read-only sessions except the explicit refresh test. Claude used its existing login with `--setting-sources` empty, strict empty MCP configuration, a session-local plugin directory, and no session persistence. Terminal tools were authorized only for these requested fixture checks; normal host trust was not bypassed. Production configuration was not changed.

Exact reusable prompts/expected outcomes are in [shared journeys](<../../../integrations/shared/journeys.md>). Raw JSONL transcripts remain under `target/agent-integrations/task52-{codex,claude}-*.jsonl`.

#### Codex observations

##### explicit

11 completed terminal operations. Final graph-context output:

Two `start` functions match. Which should I inspect for possible callees?

- `entry.ts:2 (/private<temporary-project>/entry.ts:2)` — `start(flag: boolean): number`
- `other.ts:1 (/private<temporary-project>/other.ts:1)` — `start(): number`

Queries included `observed-inputs=/`. Generation: `d976d366d31289c81e2aab7bf241f341945c1393118534d3ff614bd6730cbe82`.

Freshness is **stale**, with `input_changed` at `@repo/`, so results describe the earlier analyzed state. No refresh or file edits were performed.

##### callers

5 completed terminal operations. Final graph-context output:

`leaf(): number` at `entry.ts:1 (/private<temporary-project>/entry.ts:1)` has one caller in the graph:

- ``start(flag: boolean): number` (/private<temporary-project>/entry.ts:2)` → `leaf`, with one direct call relationship reported by `typescript.compiler`. This is source-derived evidence, not observed execution.

The Incoming query used depth 1, CLI default filters, and bounds of 100 nodes/300 edges. No results were omitted; no diagnostics were reported.

Freshness: `match`. Generation: `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. Every query used `observed-inputs=/`. No refresh or file edits were performed.

##### repeat

1 completed terminal operations. Final graph-context output:

Unchanged (generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`, freshness `match`); see the previous caller summary.

##### flow

5 completed terminal operations. Final graph-context output:

`Contract.run(): void` at `entry.ts:3 (/private<temporary-project>/entry.ts:3)` is an interface method declaration with no executable body. Its signature accepts no arguments and returns `void`; there is no implementation flow to explain.

The flow query returned `flow_unavailable`: “No source body is available for this declaration.”

Freshness: `match`, generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. Every query retained `observed-inputs=/`. No refresh or edits were performed.

##### stale

1 completed terminal operations. Final graph-context output:

Current impact is uncertain: the graph is now **stale** because `@repo/other.ts` changed (`input_changed`).

In the earlier analyzed state, `start(flag: boolean): number` directly called `leaf(): number`. Editing `leaf`’s return value or behavior could therefore change `start`’s result; changing its signature could require updating that caller.

That historical Incoming query used depth 1, default filters, and bounds of 100 nodes/300 edges, with no omissions. It does not establish the current caller set or wider impact.

Generation: `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. The status check used the same explicit project and `observed-inputs=/` mapping. No refresh or edits were performed.

##### refresh

1 completed terminal operations. Final graph-context output:

Refresh failed: `ccnavigator analyze` returned `Operation not permitted (os error 1)` in this read-only environment.

The graph remains stale because `@repo/other.ts` changed. Its earlier caller summary showed `start(flag: boolean): number` directly calling `leaf()`, but that is historical evidence, not a verified current caller list.

##### bounded

4 completed terminal operations. Final graph-context output:

The bounded Outgoing query for `entry.ts:2`’s `start(flag: boolean): number` returned only `start` itself. The one-node limit excluded the callee: **1 node and 1 edge were omitted**, and `truncated: true`. This does not mean there are no callees.

Earlier historical evidence identified a direct call to `leaf()`, but this bounded result does not display it.

Scope: depth 1, CLI default filters, maximum **1 node / 1 edge**. The graph is **stale** (`input_changed: @repo/other.ts`), generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. All queries retained the same explicit project and `observed-inputs=/` mapping. No refresh was performed.

##### refresh-authorized

11 completed terminal operations. Final graph-context output:

Refreshed using the discovered `tsconfig.json` target and explicit root `observed-inputs=/`. No source was edited.

Generation: `1cfb84f22f0fda393474a50692143383c52a0e658cb887c0b157803cd750a5e8`
Freshness: **match**.

`leaf(): number` at [entry.ts:1](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/temporary-project>/entry.ts:1) has one identified caller: `start(flag: boolean): number` at line 2. The TypeScript compiler resolved one direct call from `start` to `leaf`; this is structural evidence, not observed execution.

The Incoming query used depth 1, CLI default filters, and bounds of 100 nodes/300 edges. Nothing was omitted or truncated; no diagnostics were reported.

##### missing

4 completed terminal operations. Final graph-context output:

I couldn’t identify callers of `leaf` in the requested project: `ccnavigator` returned `graph_missing` (“No such file or directory”). Generation and freshness are both unavailable.

Source fallback found no files. This does not establish that `leaf` has no callers. No analysis or refresh was run.

#### Claude observations

Explicit invocation independently presented both start candidates without selecting a first hit (`task52-claude-explicit.jsonl`). The three-turn callers/repeat/flow conversation reported match freshness and used the following observable behavior:

##### Turn 1 (4 tool calls)

`leaf` has one caller in the graph: **`start`**.

**Graph state:** project `<temporary-project>`, with the root mapping `observed-inputs` → `/` used in every query. Generation is `cd66088bf817…682ed4` and freshness is **`match`**, meaning the graph agrees with the inputs the tool could check, with no reasons listed. The whole graph has 6 nodes and 1 edge. I did not refresh or edit anything.

**Target:** the search found only one `leaf`, so there was nothing to choose between. It is `leaf(): number`, a function declared at `entry.ts:1` and built from `tsconfig.json`.

**Callers** (looking one step up, at what calls `leaf`; no filters added, limits of 100 nodes and 300 edges):

| Caller | Location | Edge |
|---|---|---|
| `start(flag: boolean): number` | `entry.ts:2` | direct call, seen once, found by the TypeScript compiler |

- Nothing was cut off (truncated: false, 0 omitted nodes or edges) and there were no diagnostics.
- Neither function is a test or generated code.
- Only one level of callers was checked, so I didn't look for anything that calls `start`.

**Limits:** this edge comes from reading the source, not from watching the program run. `start` takes a `flag: boolean`, so the call to `leaf` might only happen on one branch. The caller list doesn't show that. A `flow` query on `start` would answer it if you want.

##### Turn 2 (1 tool calls)

Nothing has changed (generation `cd66088bf817…682ed4`, freshness `match`), so the previous caller summary still applies.

##### Turn 3 (4 tool calls)

The graph can't show a flow for `Contract.run`. It is declared in an interface, so it has no body to trace.

**Graph state:** same project, root mapping `observed-inputs` → `/`, generation `cd66088bf817…682ed4`. Freshness is **`match`**, with no reasons listed. I did not refresh anything.

**Target:** the search for `run` found only one method, so there was nothing to choose between:
- `run(): void` at `entry.ts:3`. It is declared as a method signature inside the interface `Contract`, built from `tsconfig.json`.
- The symbol details say `body_unavailable: true`. It is not a test or generated code.

**Flow result:** the `flow` query returned the error **`flow_unavailable`**: "No source body is available for this declaration." (exit 2, no result). This doesn't mean the flow is empty. An interface method only declares a type and contains no code. The actual flow would be inside whatever classes or objects implement `Contract`.

To build the requests I wrote a temporary file in `/tmp`, then deleted it. Nothing in the project was changed.

**Possible next steps** (I haven't run these):
- Look at incoming relationships for `run` to see what calls it or implements it, then run `flow` on an implementation that has a body.
- Read `entry.ts` directly. The source would show what implements `Contract`, but that isn't the same as graph-based flow.

#### Iteration and remaining package gates

The first Codex trial guessed the linked-root JSON shape incorrectly; the canonical workflow now includes exact array-of-pairs and selection examples. An early repeat correctly skipped the section request but restated caller content; the refined instruction and retest returned only a reference plus generation/freshness. The read-only refresh trial explicitly reported its permission failure; the subsequent workspace-write, explicitly authorized refresh succeeded and returned a new match generation. Diagnostic transcripts are retained separately.

These representative conversations are behavioral acceptance, not a guarantee of all future model outputs. Full installation/uninstallation and implicit selection belong to TASK-53/54. Desktop UI, other platforms and hook behavior are not claimed.

#### Missing-command fallback

A separate real Claude session without the test binary directory on PATH reported command-not-found/exit 127, no graph generation/freshness, no installation or analysis, and clearly labeled its caller answer as source-derived. Raw transcript: `target/agent-integrations/task52-claude-unavailable.jsonl`. The caller evidence was correct, but the model also misplaced an unrelated interface declaration in its prose. The skill now requires filename/line-preserving source fallback instead of anonymous concatenated file content; this model-output limitation is not hidden as a protocol success.

#### Independent transcript review

The tester independently confirmed generation binding in analyzed callers/flow/bounded queries, one status-only repeat per host, preserved flow_unavailable and no implicit analysis in stale/missing cases. Codex repeat was 227 UTF-8 bytes; all checked final graph-context outputs were below 8 KiB. The authorized-refresh trial omitted the test root mapping on its first status attempt, then restored it for analysis and subsequent queries; it does not establish perfect mapping retention on every initial attempt. The final source-fallback wording awaits its package-level retest.

#### Final verification

`./verify-full.sh` passed all 28 stages in `target/verification/task52-direct-full/`: 552 ordinary Rust tests, 642 release tests including ignored acceptance, 164 Python project tests, 26 Python tool tests and 112 TypeScript tests; storage/GPU benchmarks, packaging/signature verification, independent MCP installation, CodeScene and cleanup passed. The newly packaged ccnavigator then passed all 20 direct CLI journey operations via PATH (`target/agent-integrations/task52-packaged-cli.json`), exercising adjacent bundled analyzer resources. All seven Code Health eligible changed source files passed the pre-commit safeguard; reviewed files scored 10.0. Sonar is inactive because this checkout has no `.sonar/config.json`.


<a id="source-evidence-agent-integrations-hook-spikes-md"></a>

## evidence/agent-integrations/hook-spikes.md

Source: [evidence/agent-integrations/hook-spikes.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/hook-spikes.md). Historical record; retained rationale, methods and limitations.

### Optional graph freshness hook spikes: TASK-55 and TASK-56

2026-11-13, macOS. **Initial feasibility evidence, not completed host acceptance.**
Observed CLI versions: Claude Code 2.1.306 and codex-cli 0.168.0. The latter
is not a desktop version measurement. No production hook was installed and no
integration package was changed.

#### Current documentation and prototype boundary

Official sources opened on this date:

- [Claude hook reference](https://code.claude.com/docs/en/hooks): successful
  `PostToolUse` edits expose explicit `tool_input.file_path`; JSON can add
  advisory context. Current common inputs document `prompt_id`. Its presence
  in the installed Claude version remains unverified.
- [Codex hook reference](https://learn.chatgpt.com/docs/hooks): `apply_patch`
  uses `tool_input.command`, with `turn_id`; post-tool JSON can add context.
  Non-managed hooks require review of the exact definition; changed definitions
  require review again.
- [Codex plugin packaging](https://developers.openai.com/plugins/build/plugins)
  and [Claude plugin submission guidance](https://developers.openai.com/plugins/guides/submit-claude-plugin):
  hook-dependent distribution targets manual desktop installation. This report
  makes no public hook-distribution or CLI plugin-hook parity claim.

The disposable Python prototype took host, explicit project root and metadata
folder as arguments. It accepted Claude `Edit`/`Write` absolute paths or Codex
`apply_patch` patch headers, checked canonical containment of all named paths
including rename destinations, and ignored shell, MCP and unknown inputs.
It emitted only `hookSpecificOutput` with `hookEventName` and `additionalContext`:

> Before relying on graph context after this edit, check graph_status and report freshness; refresh only with explicit user authorization.

A file lock serialized deduplication by session and turn (`prompt_id` for Claude,
`turn_id` for Codex). Missing identities produced no reminder. State contained
hashed identity metadata only; there was no graph read, cache, query, refresh,
blocking decision or argument rewrite. Exercised malformed-input, metadata-I/O and path-resolution failures produced
no advisory and exit zero. This was a feasibility handler, not a complete patch parser or released
hook artifact. It was not loaded by either host. The scanner deliberately supports only a
conservative subset of patch structure, so unrecognized valid edit forms can
miss reminders; no broad patch-parser coverage is claimed.

#### Direct subprocess observations

Python 3.9.6. An empty handler first failed the observable explicit-edit assertion.
After implementation, **41 checks passed** against real subprocess stdout,
exit status and JSON shape:

| Cases | Observed result |
| --- | --- |
| Explicit edit / Claude Write / next turn | Advisory emitted; text below 512 characters |
| Repeated edit in same turn | No second advisory |
| Eight concurrent duplicate Codex events | Exactly one advisory |
| Shell, MCP, unknown tools, non-post event | No output |
| Missing session/turn/cwd, malformed JSON, non-object/unknown input | No output |
| Outside paths, traversal, symlink escape, outside rename, mixed inside/outside patch | No output |
| Unknown patch marker, header-only Add patch or non-patch command | No output |
| Symlink cycle | No output, exit zero |
| Supported update with `*** End of File` | Advisory emitted |
| Metadata path is a file instead of a directory | No output, exit zero |

Thirty sequential eligible Codex-handler process launches measured with
`perf_counter_ns` had median **33.668 ms**, minimum 28.873 ms, maximum 44.077 ms.
These include Python startup and metadata I/O; they are **not** host delivery
latency, model overhead or evidence of usefulness. Timeout and failed-hook host
behavior were not exercised by these bounded subprocess checks.

Temporary working artifacts were created under
`/private/tmp/ccnavigator-hook-spike.XNEobg` for review and removed after independent testing.
Final handler SHA-256:
`2ee9c752fece04d3fc0f3705a9247243934ebce58f947004425de691e80686d8`.
No executable prototype is retained in this repository. The results above are
a record of a disposable experiment; this report does not claim the removed
prototype can be reproduced byte-for-byte from retained source.

#### Host validation limits and next steps

| Required observation | TASK-55 Claude | TASK-56 Codex desktop |
| --- | --- | --- |
| Normal-trust reminder delivery | Not tested | Unavailable in this session |
| Untrusted/disabled hook | Not tested | Unavailable in this session |
| Changed-definition review | Not applicable as Codex-specific criterion | Unavailable in this session |
| Failed/timed-out hook; removal | Not tested | Unavailable in this session |
| Same editing journey with/without reminder | Not tested | Unavailable in this session |
| Model freshness checks and delivery timing | Not measured | Not measured |

The normal Claude authentication probe reported `loggedIn: true`; the isolated
`CLAUDE_CONFIG_DIR=/private/tmp/ccn-hook-host-5ce0kjul` probe reported
`loggedIn: false`, `authMethod: none`, exit 1. No credentials were copied, login
started or global configuration changed. An authenticated isolated interactive
Claude session is therefore still needed. Print mode cannot establish ordinary
interactive project trust. Missing `prompt_id` on the installed version would
require a separately validated turn-boundary design, not session-only suppression.

Native desktop computer control is disabled in the available UI tool; it supports
browser surfaces only. No documented independent desktop profile/instance
isolation was established. The current Codex app context does not expose a
verified version or plugin trust UI. These limits prevent observing desktop
installation, definition review, delivery, disablement and removal here; CLI or
direct-handler success cannot substitute for them.

For each host, continue in an isolated authenticated fixture with the existing
native graph-context skill and project-bound MCP server. Establish graph status,
then ask for a small `entry.ts` edit followed by an impact explanation, without
requesting refresh. Repeat with and without the reminder using identical
starting graph/source bytes. Record actual edit payload, hook start/completion,
model-visible advisory, subsequent `graph_status`, freshness and any refresh
attempt. Repeat edits in one turn and a new turn. Then test disabled/untrusted,
failure, timeout and removal; Codex additionally needs definition modification
and re-review. Keep normal host trust and approvals throughout.

**Recommendation: do not release either hook yet.** Handler feasibility and
small process cost do not demonstrate model benefit. TASK-55 still requires its
real comparison and normal-trust negative cases; TASK-56 may record unavailable
validation, but desktop behavior and usefulness remain unproven. Keep both
production packages hook-free.


<a id="source-evidence-agent-integrations-hosts-md"></a>

## evidence/agent-integrations/hosts.md

Source: [evidence/agent-integrations/hosts.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/hosts.md). Historical record; retained rationale, methods and limitations.

### Isolated host configuration probes

2026-11-12; codex-cli 0.166.0; Claude Code 2.1.306. Temporary CODEX_HOME and CLAUDE_CONFIG_DIR; no production configuration changed. These are configuration/manifest checks, not model invocation or interactive hook-trust tests.


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/hosts.md).



<a id="source-evidence-agent-integrations-native-mcp-hosts-initial-md"></a>

## evidence/agent-integrations/native-mcp-hosts-initial.md

Source: [evidence/agent-integrations/native-mcp-hosts-initial.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/native-mcp-hosts-initial.md). Historical record; retained rationale, methods and limitations.

### Initial native-MCP Claude host evidence

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

#### Observed tool counts

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

#### Explicit-labelled ambiguity case

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

#### Implicit-selection caller case

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

#### Disabled-MCP source fallback

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

#### Final source wrapper consistency

The current repository wrappers were independently checked with the package
validator and direct byte comparison against its generated header plus the
canonical body. This is static package consistency evidence; it does not prove
which exact source bytes were present at an earlier trace timestamp.

Canonical body SHA-256: `49d7b3804302e3a006e156ebdc17502dda73965c09160af11375a6634008bf32`.

| Host source | Result | Current wrapper SHA-256 |
| --- | --- | --- |
| claude-code | exact generated wrapper match | `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5` |
| codex | exact generated wrapper match | `1f216dc1441192d43c14d586ecb6e1c0f0f0ef595964300f736f6b21e55262a5` |


<a id="source-evidence-agent-integrations-protocol-md"></a>

## evidence/agent-integrations/protocol.md

Source: [evidence/agent-integrations/protocol.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/protocol.md). Historical record; retained rationale, methods and limitations.

### Real compiler MCP probe

2026-11-12; Node 24.15.0, TypeScript 6.0.3; packaged engine and MCP, JSON-line stdio.


Raw transcript/output omitted; see the [dated source record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/agent-integrations/protocol.md).
