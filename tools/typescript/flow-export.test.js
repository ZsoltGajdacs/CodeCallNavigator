'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const cp=require('node:child_process');
const modules=process.env.CODECALLNAVIGATOR_TS_NODE_MODULES || (process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
function analyze(t,source,extra={}) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-flow-ts-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  fs.symlinkSync(modules,path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',strict:true,types:[],noEmit:true},files:['main.ts',...Object.keys(extra)]}));
  fs.writeFileSync(path.join(root,'main.ts'),source);
  for(const [file,content] of Object.entries(extra)) fs.writeFileSync(path.join(root,file),content);
  const output=path.join(root,'flow.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),output],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(output,'utf8'));
}
function functions(graph) {assert.equal(graph.flow_inventory.version,1);return graph.flow_inventory.functions;}
function named(graph,name) {const id=graph.nodes.find(n=>n.label===name).id;return functions(graph).find(f=>f.symbol===id);}
function* objects(root) {
  const pending=[root];
  while(pending.length) {
    const value=pending.pop();
    if(!Array.isArray(value)) yield value;
    pending.push(...Object.values(value).filter(child=>child!==null&&typeof child==='object'));
  }
}
test('flow export preserves compiler identities, original byte spans and callback ownership',t=>{
  const source='// 猫 😀\r\nfunction leaf(x:number){return x+1}\r\nexport function run(x:boolean){if(x) return leaf(1);return [1].map(()=>leaf(2));}\r\n';
  const graph=analyze(t,source);
  const run=named(graph,'run');
  assert.equal(run.language,'typescript');
  assert.match(Buffer.from(source).subarray(run.span.start,run.span.end).toString(),/^export function run/);
  assert.equal(run.body[0].kind,'if');
  assert.equal(run.body[0].then_branch[0].value.calls[0].targets[0].symbol,graph.nodes.find(n=>n.label==='leaf').id);
  assert.equal(run.body[1].value.calls.some(c=>c.label==='leaf'),false);
  const callback=functions(graph).find(f=>graph.nodes.find(n=>n.id===f.symbol).kind==='callback');
  assert.equal(callback.body[0].value.calls[0].targets[0].symbol,graph.nodes.find(n=>n.label==='leaf').id);
});
test('flow IR retains finally, labeled loop updates and switch fallthrough',t=>{
  const graph=analyze(t,'function cleanup(){} export function run(x:number){ outer: for(let i=0;i<3;i++){try { if(x) continue outer; switch(i){case 0:x++;case 1:break;default:return x;} } finally { cleanup(); }} return x; }');
  const body=named(graph,'run').body;
  const loop=body.find(s=>s.kind==='loop');
  assert.equal(loop.label,'outer');
  assert.equal(loop.update.length,1);
  const attempt=loop.body[0];
  assert.equal(attempt.kind,'try');
  assert.equal(attempt.finally[0].value.calls[0].label,'cleanup');
  assert.equal(attempt.body[0].then_branch[0].kind,'continue');
  assert.equal(attempt.body[1].cases[0].falls_through,true);
});
test('short-circuit and ternary conditions retain conditional call placement',t=>{
  const graph=analyze(t,'function leaf(){return true} export function run(x:boolean){if(x && leaf()) return x ? leaf() : false; return false;}');
  const branch=named(graph,'run').body[0];
  assert.equal(branch.condition.kind,'and');
  assert.equal(branch.condition.left.calls.length,0);
  assert.equal(branch.condition.right.calls[0].label,'leaf');
  assert.equal(branch.then_branch[0].value.kind,'conditional');
});

test('expression statements and ordered initializers preserve short-circuit structure',t=>{
  const graph=analyze(t,'function left(){return 1} function right(){return 2} export function run(flag:boolean){flag && left();const a=flag?left():right(),b=left();return a+b;}');
  const body=named(graph,'run').body;
  assert.equal(body[0].kind,'evaluate');
  assert.equal(body[0].value.kind,'and');
  assert.equal(body[1].value.kind,'conditional');
  assert.equal(body[2].text,'Bind a');
  assert.equal(body[3].value.calls[0].label,'left');
  assert.equal(body[4].text,'Bind b');
});
test('iterables evaluate once and parameter defaults are conditional',t=>{
  const graph=analyze(t,'function source(){return [1,2]} function fallback(){return 0} export function run(x=fallback()){for(const item of source()){x+=item}return x;}');
  const body=named(graph,'run').body;
  assert.equal(body[0].kind,'if');
  assert.equal(body[0].then_branch[0].value.calls[0].label,'fallback');
  assert.equal(body[1].value.calls[0].label,'source');
  assert.equal(body[2].kind,'loop');
  assert.equal(body[2].condition.calls.length,0);
});
test('same-line overload and conditional call sites preserve distinct bindings and uncertainty',t=>{
  const graph=analyze(t,'function f(x:string):void;function f(x:number):void;function f(x:unknown){} function a(){} function b(){} export function run(flag:boolean){f(1);f("x");const chosen=flag?a:b;chosen();}');
  const body=named(graph,'run').body;
  const sites=body.filter(s=>s.kind==='evaluate').flatMap(s=>s.value.calls||[]);
  assert.equal(sites[0].targets[0].symbol,sites[1].targets[0].symbol);
  assert.notEqual(sites[0].span.start,sites[1].span.start);
  const chosen=sites.find(site=>site.label==='chosen');
  assert.equal(chosen.targets.length,2);
  assert.ok(chosen.targets.every(target=>target.certainty==='possible'));
});
test('labeled non-loop blocks retain jump scope and async scheduling is explicit',t=>{
  const graph=analyze(t,'export async function run(flag:boolean){ outer: if(flag){break outer;} await Promise.resolve(1);return 2;}');
  const flow=named(graph,'run');
  assert.equal(flow.body[0].kind,'block');
  assert.equal(flow.body[0].label,'outer');
  assert.ok(flow.diagnostics.some(message=>message.includes('Await')));
});
test('nested conditional call arguments keep lexical order and invocation attribution',t=>{
  const graph=analyze(t,'function a(){return 1} function b(){return 2} function invoke(value:number){return value} export function run(flag:boolean){return invoke(flag?a():b());}');
  const value=named(graph,'run').body[0].value;
  assert.equal(value.kind,'sequence');
  assert.equal(value.items[1].kind,'conditional');
  assert.equal(value.items[1].when_true.calls[0].label,'a');
  assert.equal(value.items[1].when_false.calls[0].label,'b');
  assert.equal(value.items[2].calls[0].label,'invoke');
  assert.equal(value.items[2].calls.length,1);
});
test('accessor flow targets reuse analyzer getter and setter policy',t=>{
  const graph=analyze(t,'class Box{get value(){return 1}set value(x:number){}}export function run(box:Box){box.value++;return box.value;}');
  const body=named(graph,'run').body;
  const update=body[0].value.calls[0];
  assert.equal(update.targets.length,2);
  assert.ok(update.targets.every(target=>target.certainty==='direct'));
  const getters=new Set(graph.edges.filter(edge=>edge.kind==='accessor'&&graph.nodes[edge.from].label==='run').map(edge=>graph.nodes[edge.to].id));
  assert.deepEqual(new Set(update.targets.map(target=>target.symbol)),getters);
  assert.equal(body[1].value.calls[0].targets.length,1);
});
test('deep source regions become partial IR without changing successful compiler graphs',t=>{
  const depth=160;
  const graph=analyze(t,`export function run(flag:boolean){${'if(flag){'.repeat(depth)}return 1;${'}'.repeat(depth)}return 0;}`);
  const flow=named(graph,'run');
  assert.ok(flow.diagnostics.some(message=>message.includes('budget')));
  const nesting=value=>value&&typeof value==='object'?1+Math.max(0,...Object.values(value).map(nesting)):0;
  assert.ok(nesting(flow)<100);
  assert.ok(graph.nodes.some(node=>node.label==='run'));
});
test('object, template and unary nested branches remain conditional in exported IR',t=>{
  const graph=analyze(t,'function a(){return 1} function b(){return 2} export function run(flag:boolean){const object={value:flag?a():b()};const text=`value ${flag?a():b()}`;return !(flag?a():b());}');
  const body=named(graph,'run').body;
  const expressions=body.filter(statement=>statement.kind==='evaluate').map(statement=>statement.value);
  for(const value of [...expressions,body.at(-1).value]) {
    assert.equal(value.kind,'sequence');
    assert.equal(value.items[0].kind,'conditional');
    assert.equal(value.items.at(-1).calls.length,0);
  }
});
test('conditional iterable alternatives are evaluated before the loop once',t=>{
  const graph=analyze(t,'function first(){return [1]} function second(){return [2]}export function run(flag:boolean){for(const item of flag?first():second()){if(item)return item}return 0;}');
  const body=named(graph,'run').body;
  assert.equal(body[0].kind,'evaluate');
  assert.equal(body[0].value.kind,'conditional');
  assert.equal(body[1].kind,'loop');
  assert.equal(body[1].condition.calls.length,0);
});
test('switch clauses retain effectful tests and lexical default placement',t=>{
  const graph=analyze(t,'function marker(){return 2}export function run(x:number){switch(x){default:return 1;case marker():return 2;}}');
  const cases=named(graph,'run').body[0].cases;
  assert.equal(cases[0].is_default,true);
  assert.equal(cases[0].test,null);
  assert.equal(cases[1].test.calls[0].label,'marker');
  assert.ok(cases[1].span.end>cases[1].span.start);
});
test('opaque nullish and optional chains do not expand skipped nested arguments',t=>{
  const graph=analyze(t,'function a(){return 1}function b(){return 2}export function run(flag:boolean,value:number|null,maybe:((x:number)=>number)|undefined){value??(flag?a():b());maybe?.(flag?a():b());value||=flag?a():b();}');
  const flow=named(graph,'run');
  assert.ok(flow.body.every(statement=>statement.value.kind==='value'));
  assert.ok(flow.diagnostics.some(message=>message.includes('Nullish')));
  assert.ok(flow.diagnostics.some(message=>message.includes('Optional')));
  assert.ok(flow.diagnostics.some(message=>message.includes('Logical-assignment')));
});

test('compiler flags classify ordinary, Promise, generator and nested function execution',t=>{
  const graph=analyze(t,'function plain(){return 1} function promised(){return Promise.resolve(1)} async function scheduled(){return 1} function* generator(){yield 1} async function* mixed(){yield 1} function outer(){async function nested(){await Promise.resolve(1)} return 1} class Box { method(){return 1} get value(){return 1} constructor(){} } const callback=()=>1; export {plain,promised,scheduled,generator,mixed,outer,Box,callback};');
  for(const name of ['plain','promised','outer','method']) assert.equal(named(graph,name).execution,'synchronous');
  assert.equal(named(graph,'generator').execution,'deferred');
  assert.equal(named(graph,'value').execution,undefined);
  for(const name of ['scheduled','mixed','nested','callback']) assert.equal(named(graph,name).execution,undefined);
  for(const flow of functions(graph)) {
    const kind=graph.nodes.find(node=>node.id===flow.symbol).kind;
    if(['constructor','get','set','accessor','callback'].includes(kind)) assert.equal(flow.execution,undefined);
  }
});
test('simple ordinary function calls prove only their isolated final invocation',t=>{
  const graph=analyze(t,'function leaf(x=0){return x} export function run(parameter:number){leaf();leaf(1);return leaf(parameter);}');
  const body=named(graph,'run').body;
  for(const statement of body) {
    assert.equal(statement.value.kind,'value');
    assert.equal(statement.value.calls.length,1);
    const site=statement.value.calls[0];
    assert.deepEqual(site.invocation,{timing:'synchronous',dispatch:'direct',isolated_ordered:true});
    assert.deepEqual(site.span,statement.value.span);
  }
});
test('flattened nested calls remain unproven but existing ordered conditional finalizers can prove',t=>{
  const graph=analyze(t,'function leaf(x=0){return x} export function run(flag:boolean){leaf(leaf(1));return leaf(flag?leaf(1):leaf(2));}');
  const body=named(graph,'run').body;
  assert.equal(body[0].value.kind,'value');
  assert.ok(body[0].value.calls.every(site=>!site.invocation));
  const value=body[1].value;
  assert.equal(value.kind,'sequence');
  assert.equal(value.items.at(-1).calls[0].invocation?.isolated_ordered,true);
});

test('runtime import aliases resolve one authored overload implementation',t=>{
  const graph=analyze(t,'import {leaf as alias} from "./leaf"; export function run(parameter:number){return alias(parameter);}',
    {'leaf.ts':'export function leaf(x:number):number; export function leaf(x:number){return x;}'});
  assert.equal(named(graph,'run').body[0].value.calls[0].invocation?.isolated_ordered,true);
});
test('declaration-only callees remain unknown and type-only calls fail compiler validation',t=>{
  const declaration={'ambient.d.ts':'export declare function leaf(x:number):number;'};
  const graph=analyze(t,'import {leaf} from "./ambient"; export function run(){return leaf(1);}',declaration);
  assert.equal(named(graph,'run').body[0].value.calls[0].invocation,undefined);
  assert.throws(()=>analyze(t,'import type {leaf} from "./ambient"; export function run(){return leaf(1);}',declaration));
});
test('allocations, optional, property, getter, computed, spread and callback forms stay unproven',t=>{
  const graph=analyze(t,'function leaf(value:any){return value} function tag(parts:TemplateStringsArray){return ""} class Box { get f(){return leaf} method(x:any){return x} } export function run(box:Box,callback:(x:number)=>number){box.method(1);box.f(1);box["method"](1);callback(1);leaf?.(1);leaf(...[1] as const);leaf(/x/);leaf({});leaf([]);leaf(new Box());leaf(()=>1);new Box();tag`x`;}');
  const sites=[...objects(named(graph,'run').body)].filter(value=>value.targets);
  assert.ok(sites.length);
  for(const site of sites) assert.equal(site.invocation,undefined);
});
test('reassigned function bindings reuse shared write uncertainty instead of claiming direct proof',t=>{
  const graph=analyze(t,'function leaf(x:number){return x} function other(x:number){return x+1}\n// @ts-ignore JavaScript function binding reassignment\nleaf=other;export function run(){return leaf(1);}');
  assert.equal(named(graph,'run').body[0].value.calls[0].invocation,undefined);
});

test('generator parameter defaults are a synchronous prefix before a deferred body',t=>{
  const runtime=cp.execFileSync(process.execPath,['-e',`const events=[];function* sample(v=events.push('default')){events.push('body');yield v;}const it=sample();const afterCall=[...events];it.next();console.log(JSON.stringify({afterCall,afterNext:events}));`],{encoding:'utf8'});
  assert.deepEqual(JSON.parse(runtime),{afterCall:['default'],afterNext:['default','body']});
  const graph=analyze(t,'const events:string[]=[];export function* sample(v=events.push("default")){events.push("body");yield v;}');
  const flow=named(graph,'sample');
  assert.equal(flow.body[0].kind,'if');
  assert.equal(flow.execution,undefined);
});
