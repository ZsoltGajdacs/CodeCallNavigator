'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const cp=require('node:child_process');
const modules=process.env.CODECALLNAVIGATOR_TS_NODE_MODULES||(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT&&path.dirname(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT))||path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
function analyze(t,source) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-callable-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  fs.symlinkSync(modules,path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',strict:true,skipLibCheck:true,types:[]},include:['**/*.ts']}));
  const files=typeof source==='string'?{'main.ts':source}:source;
  for(const [file,text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(root,file)),{recursive:true});
    fs.writeFileSync(path.join(root,file),text);
  }
  const output=path.join(root,'result.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),output],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(output,'utf8'));
}
function inferred(graph,caller) {
  return graph.edges.filter(edge=>edge.provenance==='codecallnavigator.callable.v1'&&(!caller||graph.nodes[edge.from].label===caller));
}
function targets(graph,caller) {return [...new Set(inferred(graph,caller).map(edge=>graph.nodes[edge.to].label))].sort();}
function flowCalls(value) {
  if(!value||typeof value!=='object') return [];
  const own=Array.isArray(value.targets)&&value.span&&value.label?[value]:[];
  return own.concat(Object.values(value).flatMap(flowCalls));
}

test('real compiler connects immutable logical aliases, direct parameters and per-call returned values',t=>{
  const source=fs.readFileSync(path.resolve(__dirname,'../../fixtures/typescript-corpus/src/callables.ts'),'utf8');
  const graph=analyze(t,source);
  for(const [caller,expected] of [
    ['logicalNullish',['nullishFallback','nullishPrimary']],['logicalOr',['orFallback','orPrimary']],
    ['immutableAlias',['aliasFallback','aliasPrimary']],['invokeParameter',['parameterLeaf']],
    ['returnedA',['returnLeafA']],['returnedB',['returnLeafB']],['immediateReturned',['returnLeafA']],
  ]) assert.deepEqual(targets(graph,caller),expected,caller);
  for(const caller of ['identity','relay','forwardOnly','forwardingOnly','mutatedParameter','mutableAliasNegative','mutableReturnNegative']) {
    assert.deepEqual(targets(graph,caller),[],caller);
  }
  assert.ok(graph.edges.some(edge=>graph.nodes[edge.from].label==='returnedA'&&graph.nodes[edge.to].label==='identity'&&edge.certainty==='direct'));
  assert.ok(graph.edges.some(edge=>graph.nodes[edge.from].label==='callParameter'&&graph.nodes[edge.to].label==='parameterLeaf'&&edge.kind==='reference'));
});
test('logical short circuit excludes unreachable fallback functions and preserves graph/flow agreement',t=>{
  const graph=analyze(t,`const choose=Date.now()>0;function a(){return 1}function b(){return 2}function c(){return 3}
    export function run(){const maybe=choose?a:undefined;const chosen=maybe??b;const again=chosen??c;return again()}
    export function truthy(){const chosen=a||b;return chosen()}
    export function falsy(){const chosen=(false as false|(()=>number))||c;return chosen()}`);
  assert.deepEqual(targets(graph,'run'),['a','b']);
  assert.deepEqual(targets(graph,'truthy'),['a']);
  assert.deepEqual(targets(graph,'falsy'),['c']);
  for(const node of graph.nodes.filter(node=>['run','truthy','falsy'].includes(node.label))) {
    const fn=graph.flow_inventory.functions.find(fn=>fn.symbol===node.id);
    const flow=flowCalls(fn).flatMap(call=>call.targets).filter(target=>target.provenance==='codecallnavigator.callable.v1');
    assert.deepEqual([...new Set(flow.map(target=>graph.nodes.find(node=>node.id===target.symbol).label))].sort(),targets(graph,node.label));
    assert.ok(flow.every(target=>target.certainty==='possible'));
  }
});
test('direct parameter forwarding converges while defaults use each actual call environment',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}function c(){return 3}
    function downstream(cb:()=>number){return cb()}function upstream(cb:()=>number){return downstream(cb)}
    function defaults(first:()=>number,cb:()=>number=first){return cb()}
    export function start(){upstream(a);upstream(b);defaults(a,b);defaults(c)}
    function optional(cb:(()=>number)|undefined){const chosen=cb??c;return chosen()}
    export function absent(){return optional(undefined)}`);
  assert.deepEqual(targets(graph,'downstream'),['a','b']);
  assert.deepEqual(targets(graph,'defaults'),['b','c']);
  assert.deepEqual(targets(graph,'optional'),['c']);
  assert.deepEqual(targets(graph,'upstream'),[]);
});
test('simple returned function environments remain distinct through aliases, conditionals and nested identities',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function identity(cb:()=>number){const saved=cb;return saved}
    function choose(first:()=>number,second:()=>number,flag:boolean){return flag?first:second}
    export function first(){const fn=identity(identity(a));return fn()}
    export function second(){const fn=identity(b);return fn()}
    export function both(){return choose(a,b,Date.now()>0)()}`);
  assert.deepEqual(targets(graph,'first'),['a']);
  assert.deepEqual(targets(graph,'second'),['b']);
  assert.deepEqual(targets(graph,'both'),['a','b']);
  assert.deepEqual(targets(graph,'identity'),[]);
});
test('new callable parameter facts never use mutable arguments, replaced declarations or legacy React prop maps',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function invoke(cb:()=>number){return cb()}
    function changed(cb:()=>number){cb=b;return cb()}
    function replaced(){return 3}
    // @ts-expect-error deliberate runtime replacement
    replaced=b;
    export function run(){let cb=a;cb=b;invoke(cb);changed(a);invoke(replaced)}
    function property(p:{onClick:()=>number}){return invoke(p.onClick)}
    export function prop(){return property({onClick:a})}`);
  assert.deepEqual(targets(graph,'invoke'),[]);
  assert.deepEqual(targets(graph,'changed'),[]);
  assert.ok(graph.diagnostics.some(item=>item.includes('unresolved callable')));
});
test('captured returned closures and factory-local aliases are deferred, capture-free callbacks are eligible',t=>{
  const graph=analyze(t,`function a(){return 1}
    function captures(cb:()=>number){return ()=>cb()}
    function aliases(cb:()=>number){const saved=cb;return ()=>saved()}
    function free(){return ()=>a()}
    export function direct(){const fn=captures(a);return fn()}
    export function alias(){const fn=aliases(a);return fn()}
    export function independent(){const fn=free();return fn()}`);
  assert.deepEqual(targets(graph,'direct'),[]);
  assert.deepEqual(targets(graph,'alias'),[]);
  const independent=inferred(graph,'independent');
  assert.equal(independent.length,1);
  assert.equal(graph.nodes[independent[0].to].kind,'callback');
  assert.ok(graph.diagnostics.some(item=>item.includes('captures factory locals')));
});
test('spread, dynamic registries and arbitrary return wrappers retain unresolved diagnostics',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function invoke(cb:()=>number){return cb()}
    function identity(cb:()=>number){return cb}
    function noisy(cb:()=>number){Math.random();return cb}
    export function spread(){const args:[()=>number]=[a];invoke(...args);return identity(...args)()}
    export function registry(){const handlers={a,b};const key: keyof typeof handlers=Date.now()>0?'a':'b';const fn=handlers[key]??a;return fn()}
    export function arbitrary(){const fn=noisy(a);return fn()}`);
  for(const caller of ['invoke','spread','registry','arbitrary']) assert.deepEqual(targets(graph,caller),[],caller);
  assert.ok(graph.diagnostics.some(item=>item.includes('spread callable return')));
  assert.ok(graph.diagnostics.some(item=>item.includes('dynamic key')));
  assert.ok(graph.diagnostics.some(item=>item.includes('unsupported callable return wrapper')));
});
test('import aliases and source project references retain strict callable declarations',t=>{
  const compilerOptions={target:'ES2023',module:'esnext',moduleResolution:'bundler',strict:true,skipLibCheck:true,types:[],composite:true};
  const graph=analyze(t,{
    'tsconfig.json':JSON.stringify({files:[],references:[{path:'./lib'},{path:'./app'}]}),
    'lib/tsconfig.json':JSON.stringify({compilerOptions,include:['*.ts']}),
    'lib/lib.ts':'export function identity<T extends ()=>number>(callback:T){const saved=callback;return saved} export function invoke(callback:()=>number){return callback()}',
    'app/tsconfig.json':JSON.stringify({compilerOptions,include:['*.ts'],references:[{path:'../lib'}]}),
    'app/main.ts':`import {identity as pass,invoke} from '../lib/lib';function a(){return 1}function b(){return 2}
      export function first(){return pass(a)()}export function second(){return pass(b)()}export function start(){invoke(a);invoke(b)}`,
  });
  assert.deepEqual(targets(graph,'first'),['a']);
  assert.deepEqual(targets(graph,'second'),['b']);
  assert.deepEqual(targets(graph,'invoke'),['a','b']);
});
test('shadowed undefined, destructuring assignments and nested callback captures do not seed new flow',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function invoke(callback:()=>number){return callback()}
    function rewritten(callback:()=>number){({callback}={callback:b});return callback()}
    function nested(callback:()=>number){const local=()=>callback();local()}
    export function run(){let undefined:any=a;undefined=b;const selected=undefined??a;selected();rewritten(a);nested(a);invoke(undefined)}`);
  for(const caller of ['invoke','rewritten','run']) assert.deepEqual(targets(graph,caller),[],caller);
  assert.ok(graph.diagnostics.some(item=>item.includes('captured or unresolved callable parameter')));
});
test('explicit nullish default arguments defer rather than invent the return fallback',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function make(callback:(()=>number)|undefined=a){return callback??b}
    export function explicit(){return make(undefined)()}
    export function omitted(){return make()()}
    export function provided(){return make(b)()}`);
  assert.deepEqual(targets(graph,'explicit'),[]);
  assert.deepEqual(targets(graph,'omitted'),['a']);
  assert.deepEqual(targets(graph,'provided'),['b']);
  assert.ok(graph.diagnostics.some(item=>item.includes('nullish callable default argument')));
});
test('optional callable factories defer instead of dropping the skipped-call fallback',t=>{
  const graph=analyze(t,`function a(){return 1}function b(){return 2}
    function identity(callback:()=>number){return callback}
    export function run(){const factory=Date.now()>0?identity:undefined;const selected=factory?.(a)??b;return selected()}`);
  assert.deepEqual(targets(graph,'run'),[]);
  assert.ok(graph.diagnostics.some(item=>item.includes('optional callable return wrapper')));
});
