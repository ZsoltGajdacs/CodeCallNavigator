# Agent integration research evidence

2026-11-12, macOS arm64. Scope: TASK-51 research; no shipped plugin or runtime code changes.

- [Protocol transcript](protocol.md): 16 MCP tool operations plus unavailable-engine startup failure, real Node 24.15.0 / TypeScript 6.0.3 compiler fixture. Complete responses, timing and serialized response byte counts retained. Temporary project addresses normalized; no source from the dependency project analyzed.
- [Host probes](hosts.md): codex-cli 0.166.0 and Claude Code 2.1.306 isolated configuration operations and Claude minimal plugin validation, all exit zero. No model sessions started, production plugins installed, or global hooks changed.
- [Report](../../docs/agent-integrations.md): official source matrix, journeys, defaults and follow-up design.

The initial probe assertion incorrectly treated a present null error as failure; corrected to check its value. The recorded final run passed. Missing graph did not create an artifact; two same-named methods stayed ambiguous; a 1-node selection reported truncation; a body-less interface method reported unavailable flow; editing changed freshness to stale; read queries left artifact bytes identical; explicit refresh changed generation and rejected the old generation. Missing CLI startup exited nonzero. MCP process closed/reaped after each run. Initial freshness was unverifiable (unmapped_input), not fresh; the report requires preserving that distinction. No large-project benchmark claim.

## Reproduce

Run from the repository root with the separately packaged binaries present in dist/CodeCallNavigatorEngine/bin and dist/CodeCallNavigatorMcp/bin. Save each block below as a temporary Python file, replace the dependency directory in the protocol probe with a TypeScript 6.0.x node_modules installation, then run with Python 3.13 and Node 24+ on PATH. The scripts only create temporary fixture/configuration directories and rewrite these evidence Markdown files. They are research recipes, not installed automation or production test additions.

Host interactive skill invocation, full plugin lifecycle, disabled/untrusted hooks and cross-platform behavior were not tested. Registration and manifest validation must not be read as proof that a model selected or followed the proposed skill. Follow-up acceptance must cover those cases without bypassing normal host trust.

### Protocol probe

```python
import sys,json,subprocess,tempfile,pathlib,queue,threading,time,os
repo=pathlib.Path.cwd(); sys.path.insert(0,str(repo/'tools')); import mcp_smoke as m
engine=repo/'dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine'; binary=repo/'dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp'
records=[]
with tempfile.TemporaryDirectory(prefix='agent-graph-') as tmp:
 root=pathlib.Path(tmp); (root/'node_modules').symlink_to('/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules')
 (root/'tsconfig.json').write_text('{"compilerOptions":{"strict":true,"target":"ES2022","types":[]},"include":["*.ts"]}')
 (root/'entry.ts').write_text('export function leaf(): number { return 1; }\nexport function start(flag: boolean): number { if(flag) return leaf(); return 0; }\nexport interface Contract { run(): void; }\n')
 (root/'other.ts').write_text('export function start(): number { return 2; }\n')
 env=os.environ.copy(); env['CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT']='/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules/typescript'
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

### Host configuration probe

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
