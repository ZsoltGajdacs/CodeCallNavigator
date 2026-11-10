'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const cp=require('node:child_process');
const modules=process.env.CODEGRAPH_TS_NODE_MODULES || (process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT&&path.dirname(process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
function analyze(t,source) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codegraph-observer-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  fs.symlinkSync(modules,path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',strict:true,skipLibCheck:true,types:[],noEmit:true},files:['main.ts']}));
  fs.writeFileSync(path.join(root,'main.ts'),source);
  const output=path.join(root,'output.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),output],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(output,'utf8'));
}
function inferred(graph) {return graph.edges.filter(edge=>edge.provenance==='codegraph.observer.v1');}
function calls(value,result=[]) {
  if(!value||typeof value!=='object') return result;
  if(Array.isArray(value.targets)) result.push(value);
  for(const child of Object.values(value)) if(child&&typeof child==='object') calls(child,result);
  return result;
}
const moduleObserver=`type Listener=()=>void;
const callbacks=new Set<Listener>();
export function subscribe(callback:Listener){callbacks.add(callback)}
export function notify(){for(const callback of callbacks) callback()}
export function redraw(){}
subscribe(redraw);`;
test('real compiler connects observer invocation with registration evidence and matching flow targets',t=>{
  const graph=analyze(t,moduleObserver);
  const edge=inferred(graph).find(edge=>graph.nodes[edge.from].label==='notify'&&graph.nodes[edge.to].label==='redraw');
  assert.ok(edge,'notify must reach the registered callback');
  assert.equal(edge.certainty,'possible');
  assert.equal(edge.evidence[0].rule,'codegraph.observer:collection');
  assert.equal(edge.evidence[0].semantics,'possible-invocation');
  assert.equal(edge.evidence[0].invocation.line,4);
  assert.equal(edge.evidence[0].registration.line,6);
  const flow=graph.flow_inventory.functions.find(flow=>flow.symbol===graph.nodes[edge.from].id);
  const site=calls(flow).find(site=>site.targets.some(target=>target.symbol===graph.nodes[edge.to].id));
  assert.ok(site);
  assert.equal(site.targets[0].certainty,edge.certainty);
  assert.equal(site.targets[0].provenance,edge.provenance);
  assert.ok(graph.edges.some(edge=>edge.kind==='reference'&&graph.nodes[edge.to].label==='redraw'));
  assert.ok(!inferred(graph).some(edge=>graph.nodes[edge.from].label==='subscribe'));
});
function callbackEdges(graph) {return inferred(graph).map(edge=>`${graph.nodes[edge.from].label}->${graph.nodes[edge.to].label}`).sort();}
const classObserver=`type Listener=()=>void;
class Scene {
  private readonly callbacks=new Set<Listener>();
  onUpdate(callback:Listener){this.callbacks.add(callback)}
  triggerUpdate(){for(const callback of this.callbacks) callback()}
}
export const scene=new Scene();
export function redraw(){}
scene.onUpdate(redraw);`;
test('closed singleton and private static Set channels use compiler-owned declarations',t=>{
  const singleton=analyze(t,classObserver);
  assert.deepEqual(callbackEdges(singleton),['triggerUpdate->redraw']);
  const staticBus=classObserver.replace('class Scene','export class Scene').replace('private readonly','private static readonly')
    .replace('  onUpdate','  static onUpdate').replace('  triggerUpdate','  static triggerUpdate').replaceAll('this.callbacks','Scene.callbacks')
    .replace('export const scene=new Scene();','').replace('scene.onUpdate','Scene.onUpdate');
  assert.deepEqual(callbackEdges(analyze(t,staticBus)),['triggerUpdate->redraw']);
});
test('namespace-owned module channels and static fields never match by shared names',t=>{
  const source=`type Listener=()=>void;
namespace First{const callbacks=new Set<Listener>();export function subscribe(cb:Listener){callbacks.add(cb)}export function notify(){for(const cb of callbacks) cb()}}
namespace Second{const callbacks=new Set<Listener>();export function subscribe(cb:Listener){callbacks.add(cb)}export function notify(){for(const cb of callbacks) cb()}}
export function first(){} export function second(){}
First.subscribe(first);Second.subscribe(second);`;
  const graph=analyze(t,source),edges=inferred(graph);
  assert.equal(edges.length,2);
  assert.deepEqual(edges.map(edge=>[edge.evidence[0].channel,graph.nodes[edge.to].label]).sort(),[['Set:First.callbacks','first'],['Set:Second.callbacks','second']]);
});
test('multiple allocations, exported classes and constructor aliases stay unresolved',t=>{
  for(const source of [classObserver.replace('export const scene=new Scene();','const second=new Scene(); export const scene=new Scene();'),
    classObserver.replace('class Scene','export class Scene'),
    classObserver.replace('export const scene=new Scene();','const Constructor=Scene; export const scene=new Constructor();')]) {
    const graph=analyze(t,source);
    assert.deepEqual(inferred(graph),[]);
    assert.ok(graph.diagnostics.some(diagnostic=>diagnostic.includes('unresolved observer instance channels')));
  }
});
test('collection clearing, escaping and computed access disable the inferred channel',t=>{
  const variants=[moduleObserver+'\ncallbacks.clear();',moduleObserver+'\nexport function expose(){return callbacks}',
    moduleObserver.replace('callbacks.add(callback)','callbacks["add"](callback)'),
    classObserver+'\nconst key="callbacks"; (scene as any)[key].clear();'];
  for(const source of variants) {
    const graph=analyze(t,source);
    assert.deepEqual(inferred(graph),[]);
    assert.ok(graph.diagnostics.some(diagnostic=>diagnostic.includes('unresolved observer')));
  }
});
test('callback bindings must be immutable and registrar parameters unchanged',t=>{
  const variants=[moduleObserver.replace('subscribe(redraw);','let current=redraw; subscribe(current);'),
    moduleObserver.replace('callbacks.add(callback)','callback=redraw;callbacks.add(callback)'),
    moduleObserver.replace('callbacks.add(callback)','[callback]=[redraw];callbacks.add(callback)'),
    moduleObserver.replace('subscribe(redraw);','const holder={callback:redraw}; subscribe(holder.callback);')];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('registration without direct invocation and nested deferred dispatch add no call',t=>{
  const variants=[moduleObserver.replace('callback()}','console.log(callback)}'),
    moduleObserver.replace('callback()}','(()=>callback())}'),
    moduleObserver.replace('for(const callback of callbacks) callback()','callbacks.forEach(callback=>callback())')];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('loop shadowing uses exact symbols and mutable iteration bindings are deferred',t=>{
  const shadow=moduleObserver.replace('for(const callback of callbacks) callback()','for(const callback of callbacks){const callback=()=>{};callback()}');
  assert.deepEqual(inferred(analyze(t,shadow)),[]);
  const graph=analyze(t,moduleObserver.replace('for(const callback of callbacks)','for(let callback of callbacks)'));
  assert.deepEqual(inferred(graph),[]);
  assert.ok(graph.diagnostics.some(diagnostic=>diagnostic.includes('mutable iteration')));
});
test('multiple registrations retain all source evidence without inflating invocation counts',t=>{
  const graph=analyze(t,moduleObserver+'\nsubscribe(redraw);');
  const edge=inferred(graph)[0];
  assert.equal(edge.count,1);
  assert.equal(edge.evidence.length,2);
  assert.deepEqual(edge.evidence.map(item=>item.registration.line).sort(),[6,7]);
  const removed=analyze(t,moduleObserver.replace('subscribe(redraw);',''));
  assert.deepEqual(inferred(removed),[]);
});
test('custom Set and reassigned methods cannot supply observer targets',t=>{
  const custom=`class Set<T> extends globalThis.Set<T>{} export {};`+moduleObserver;
  assert.deepEqual(inferred(analyze(t,custom)),[]);
  const reassigned=classObserver+'\nscene.onUpdate=()=>{};';
  assert.deepEqual(inferred(analyze(t,reassigned)),[]);
});
test('constructor metadata and explicit untyped instance escapes fail the singleton gate',t=>{
  const variants=[classObserver.replace('onUpdate(callback:Listener)','constructor(){new.target;} onUpdate(callback:Listener)'),
    classObserver+'\nconst ctor=scene.constructor;',
    classObserver+'\nconst loose:any=scene; loose.callbacks.clear();',
    classObserver+'\nconst fake={} as Scene;',
    classObserver+'\n(scene as any).callbacks.clear();'];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('shorthand collection escape and object-destructured parameter writes are rejected',t=>{
  const variants=[moduleObserver+'\nconst exposed={callbacks};',
    moduleObserver.replace('callbacks.add(callback)','({callback}={callback:redraw});callbacks.add(callback)')];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('noncallable Sets do not produce observer diagnostics',t=>{
  const graph=analyze(t,'const values=new Set<string>();values.add("x");values.clear();export {};');
  assert.deepEqual(inferred(graph),[]);
  assert.ok(!graph.diagnostics.some(diagnostic=>diagnostic.includes('observer')));
});
test('one constructor allocation site inside a factory is not a singleton instance',t=>{
  const source=classObserver.replace('export const scene=new Scene();','function create(){return new Scene()}\nexport const scene=create();const other=create();');
  assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('each actual callback call contributes one invocation count',t=>{
  const source=moduleObserver.replace('for(const callback of callbacks) callback()','for(const callback of callbacks){callback();callback()}');
  const graph=analyze(t,source),edge=inferred(graph)[0];
  assert.equal(edge.count,2);
  assert.equal(edge.evidence.length,2);
  assert.equal(new Set(edge.evidence.map(item=>item.invocation.span.start)).size,2);
});
test('private static channels in namespace classes retain their actual owners',t=>{
  const bus=name=>`namespace ${name}{export class Bus{private static readonly callbacks=new Set<()=>void>();static subscribe(cb:()=>void){Bus.callbacks.add(cb)}static notify(){for(const cb of Bus.callbacks)cb()}}}`;
  const graph=analyze(t,bus('First')+bus('Second')+'export function first(){} export function second(){}First.Bus.subscribe(first);Second.Bus.subscribe(second);');
  assert.deepEqual(inferred(graph).map(edge=>[edge.evidence[0].channel,graph.nodes[edge.to].label]).sort(),[['Set:First.Bus.callbacks','first'],['Set:Second.Bus.callbacks','second']]);
});
test('default-library ownership cannot be spoofed by a project no-default-lib directive',t=>{
  const source='/// <reference no-default-lib="true"/>\n/// <reference lib="es2023"/>\nclass Set<T> extends globalThis.Set<T>{} export {};\n'+moduleObserver;
  assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('constructor export clauses, extracted registrar methods and collection replacement are deferred',t=>{
  const variants=[classObserver+'\nexport {Scene};',
    classObserver.replace('scene.onUpdate(redraw);','const register=scene.onUpdate;register(redraw);'),
    classObserver.replace('  onUpdate','  constructor(){this.callbacks=new Set<Listener>()} onUpdate')];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('static fields on a repeatedly created local class are not shared channels',t=>{
  const source=`export function leaf(){} function create(){class Bus{private static readonly callbacks=new Set<()=>void>();static subscribe(cb:()=>void){Bus.callbacks.add(cb)}static notify(){for(const cb of Bus.callbacks)cb()}}return Bus}const first=create(),second=create();first.subscribe(leaf);second.notify();`;
  assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('project writes to builtin Set storage operations disable inference',t=>{
  const variants=[moduleObserver+'\nSet.prototype.add=function(value){return this};',
    moduleObserver+'\nSet.prototype[Symbol.iterator]=function(){return [][Symbol.iterator]()};',
    moduleObserver+'\nSet=class<T> extends globalThis.Set<T>{};'];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('method callback inference rejects mutable receivers while accepting const literal and static owners',t=>{
  const registration='subscribe(redraw);';
  const mutable=moduleObserver.replace(registration,'let holder={method(){}};holder={method(){}};subscribe(holder.method);');
  assert.deepEqual(inferred(analyze(t,mutable)),[]);
  const constant=moduleObserver.replace(registration,'const holder={method(){}};const alias=holder;subscribe(alias.method);');
  assert.deepEqual(inferred(analyze(t,constant)).map(edge=>edge.certainty),['possible']);
  const staticOwner=moduleObserver.replace(registration,'class Handler{static method(){}}subscribe(Handler.method);');
  assert.deepEqual(inferred(analyze(t,staticOwner)).map(edge=>edge.certainty),['possible']);
  const cast=moduleObserver.replace(registration,'let holder={method(){}};const alias=holder as {method:()=>void};subscribe(alias.method);');
  assert.deepEqual(inferred(analyze(t,cast)),[]);
});
test('runtime-replacing class, field and registrar decorators leave observer channels unresolved',t=>{
  const decorator='function replace(value:any,context:any):any{return function(){}}\n';
  const variants=[decorator+classObserver.replace('class Scene','@replace class Scene'),
    decorator+classObserver.replace('  private readonly','  @replace private readonly'),
    decorator+classObserver.replace('  onUpdate','  @replace onUpdate')];
  for(const source of variants) assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('decorated callback methods or owning classes cannot supply inferred targets',t=>{
  const decorator='function replace(value:any,context:any):any{return function(){}}\n';
  const methods=['class Handler{@replace static method(){}}','@replace class Handler{static method(){}}'];
  for(const method of methods) {
    const source=decorator+moduleObserver.replace('subscribe(redraw);',method+'subscribe(Handler.method);');
    assert.deepEqual(inferred(analyze(t,source)),[]);
  }
});
test('instance registrar receivers must originate at the proven singleton const',t=>{
  const bad=classObserver.replace('scene.onUpdate(redraw);','let alias:Scene=scene;const fake:any={onUpdate(_cb:Listener){}};alias=fake;alias.onUpdate(redraw);');
  assert.deepEqual(inferred(analyze(t,bad)),[]);
  const good=classObserver.replace('scene.onUpdate(redraw);','const alias=scene;alias.onUpdate(redraw);');
  assert.deepEqual(callbackEdges(analyze(t,good)),['triggerUpdate->redraw']);
  const internal=classObserver.replace('  triggerUpdate()','  register(){this.onUpdate(redraw)} triggerUpdate()').replace('scene.onUpdate(redraw);','scene.register();');
  assert.deepEqual(callbackEdges(analyze(t,internal)),['triggerUpdate->redraw']);
});
test('structural object escapes cannot hide method replacement behind new declarations',t=>{
  const values=[
    'const alias:{method():void}=holder;alias.method=()=>{};',
    'function replace(value:{method():void}){value.method=()=>{}}replace(holder);',
    'const values:Array<{method():void}>=[holder];values[0].method=()=>{};',
    'function expose():{method():void}{return holder}expose().method=()=>{};'];
  for(const escape of values) {
    const source=moduleObserver.replace('subscribe(redraw);','const holder={method(){redraw()}};'+escape+'subscribe(holder.method);');
    assert.deepEqual(inferred(analyze(t,source)),[]);
  }
});
test('structural singleton aliases cannot hide registrar replacement',t=>{
  const source=classObserver.replace('scene.onUpdate(redraw);','const alias:{onUpdate(cb:Listener):void}=scene;alias.onUpdate=()=>{};scene.onUpdate(redraw);');
  assert.deepEqual(inferred(analyze(t,source)),[]);
});
test('dispatching instance methods may not be extracted or rebound to other receivers',t=>{
  for(const use of ['scene.triggerUpdate.call({callbacks:new Set<Listener>()});','const dispatch=scene.triggerUpdate;dispatch();']) {
    assert.deepEqual(inferred(analyze(t,classObserver+'\n'+use)),[]);
  }
});
test('static private collection access through dynamic this is deferred',t=>{
  const source=`type Listener=()=>void;class Bus{static readonly #callbacks=new Set<Listener>();static subscribe(cb:Listener){this.#callbacks.add(cb)}static notify(){for(const cb of this.#callbacks)cb()}}class Derived extends Bus{}export function leaf(){}Derived.subscribe(leaf);Bus.notify();`;
  assert.deepEqual(inferred(analyze(t,source)),[]);
});
