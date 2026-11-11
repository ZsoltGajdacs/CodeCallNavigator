const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const cp = require('node:child_process');
const engineModules = process.env.CODECALLNAVIGATOR_TS_NODE_MODULES || (process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname, '../../../PacificFateMapEditor/node_modules');
function fixture(t, files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codecallnavigator-ts-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.symlinkSync(engineModules, path.join(root, 'node_modules'), 'dir');
  for (const [name, text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(root, name)), { recursive: true });
    fs.writeFileSync(path.join(root, name), typeof text === 'string' ? text : JSON.stringify(text));
  }
  return root;
}
function run(root, config = 'tsconfig.json', mode = 'analyze') {
  const output = path.join(root, 'output.json');
  cp.execFileSync(process.execPath, [path.join(__dirname, 'analyze.js'), mode, path.join(root, config), output], { stdio: 'pipe' });
  return JSON.parse(fs.readFileSync(output, 'utf8'));
}
const config = { compilerOptions: { target: 'ES2023', module: 'esnext', moduleResolution: 'bundler', strict: true, skipLibCheck: true, jsx: 'react-jsx', types: [], noEmit: true }, include: ['src/**/*'] };
test('semantic export keys survive comments and declaration movement within a file',t=>{
  const source='export function a(){return 1}\nexport function b(){return a()}\n';
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  fs.writeFileSync(path.join(root,'src/main.ts'),'// moved declarations\nexport function b(){return a()}\n\nexport function a(){/* comment */return 1}\n');
  const after=run(root);
  const named=graph=>Object.fromEntries(graph.nodes.filter(node=>['a','b'].includes(node.label)).map(node=>[node.label,graph.export_identities[node.id]]));
  assert.deepEqual(named(before),named(after));
  assert.notDeepEqual(before.nodes.map(n=>n.id),after.nodes.map(n=>n.id));
});
test('exact semantic export identities keep module declarations and named callable kinds',t=>{
  const declarations=[
    'export function first():number{return 1}',
    'export function overloaded(value:string):string; export function overloaded(value:number):number; export function overloaded(value:string|number){return value}',
    'export class A{public run():number{return 1}}',
    'export class B{public run():number{return 2}}',
    'export function entry(){return first()}',
  ];
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':declarations.join('\n')});
  const before=run(root);
  const modules=before.nodes.filter(node=>node.kind==='module');
  assert.equal(modules.length,1);
  assert.equal(before.export_identities[modules[0].id],'<module>');
  const callables=graph=>graph.nodes.filter(node=>node.kind==='function');
  for(const node of callables(before)) {
    const base=JSON.parse(JSON.parse(before.export_identities[node.id])[0]);
    assert.equal(base[1],node.label==='run'?'MethodDeclaration':'FunctionDeclaration');
    assert.equal(base[2],node.label);
    assert.equal(base[3],node.signature);
  }
  const keys=graph=>callables(graph).map(node=>graph.export_identities[node.id]).sort();
  assert.equal(new Set(keys(before)).size,callables(before).length);
  fs.writeFileSync(path.join(root,'src/main.ts'),'// reordered and edited named bodies\n'+declarations.toReversed().join('\n').replaceAll('return 1','return 3'));
  const after=run(root);
  assert.deepEqual(keys(before),keys(after));
  assert.equal(after.export_identities[after.nodes.find(node=>node.kind==='module').id],'<module>');
  assert.deepEqual(edges(before,'entry'),edges(after,'entry'));
});
test('anonymous export keys ignore trivia and distinguish owning classes',t=>{
  const source='export class A{run(){return ()=>1}} export class B{run(){return ()=>1}}';
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  fs.writeFileSync(path.join(root,'src/main.ts'),source.replaceAll('()=>1','() => /* comment */ 1'));
  const after=run(root);
  const callbacks=graph=>graph.nodes.filter(node=>node.kind==='callback').map(node=>graph.export_identities[node.id]).sort();
  assert.equal(new Set(callbacks(before)).size,2);
  assert.deepEqual(callbacks(before),callbacks(after));
});
test('semantic keys distinguish namespace, interface and object owners and ignore implicit constructors',t=>{
  const source='export namespace A{export function run(){}} export namespace B{export function run(){}} export interface X{run():void} export interface Y{run():void} export const first={run(){}}; export const second={run(){}}; export class C{run(){}}';
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  const keys=before.nodes.filter(node=>node.label==='run').map(node=>before.export_identities[node.id]);
  assert.equal(new Set(keys).size,keys.length);
  fs.writeFileSync(path.join(root,'src/main.ts'),source.replace('class C{','class C{constructor(){}'));
  const after=run(root);
  const last=graph=>graph.nodes.filter(node=>node.label==='run').at(-1);
  assert.equal(before.export_identities[last(before).id],after.export_identities[last(after).id]);
});
test('framework evidence records distinct source sites and deferred versus referenced callbacks',t=>{
  const source='import {useEffect} from "react"; function leaf(){} export function Page(){const emoji="🧭";useEffect(()=>{leaf()},[]);return <><button onClick={leaf}/><button onClick={leaf}/></>}';
  const root=fixture(t,{'tsconfig.json':config,'src/main.tsx':source});
  const graph=run(root);
  const event=graph.edges.find(edge=>edge.kind==='react-event');
  assert.equal(event.count,2);
  assert.equal(event.evidence.length,2);
  assert.ok(event.evidence.every(item=>item.version===1&&item.semantics==='reference'));
  assert.equal(new Set(event.evidence.map(item=>item.invocation.span.start)).size,2);
  const effect=graph.edges.find(edge=>edge.kind==='react-effect');
  assert.equal(effect.evidence[0].semantics,'deferred');
  const site=effect.evidence[0].invocation;
  assert.equal(Buffer.from(source).subarray(site.span.start,site.span.end).toString(),'useEffect(()=>{leaf()},[])');
  assert.ok(effect.evidence[0].supporting_symbols.every(id=>graph.nodes.some(node=>node.id===id)));
  assert.ok(graph.edges.filter(edge=>edge.certainty==='direct').every(edge=>!edge.evidence));
});
function edges(graph, label, kind) {
  return graph.edges.filter(e => graph.nodes[e.from].label === label && (!kind || e.kind === kind)).map(e => graph.nodes[e.to].label);
}
test('class methods retain declared access without assigning visibility to functions or file groups', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.ts': `
    export class Worker {
      ordinary() { function local() {} local(); return () => {}; }
      public exposed() {}
      private hidden() {}
      protected inherited() {}
      #secret() {}
      public static staticMethod() {}
      private get value() { return 1; }
    }
    export function entry() { new Worker().ordinary(); }
    const object = { objectMethod() {} };
    export const detached = () => object.objectMethod();
  ` });
  const graph = run(root);
  const access = Object.fromEntries(graph.nodes.map(node => [node.label, node.visibility]));
  for (const name of ['ordinary', 'exposed', 'staticMethod']) assert.equal(access[name], 'public');
  for (const name of ['hidden', '#secret', 'value']) assert.equal(access[name], 'private');
  assert.equal(access.inherited, 'protected');
  for (const name of ['local', 'entry', 'detached', 'objectMethod']) assert.equal(access[name], undefined);
  assert.ok(graph.nodes.filter(node => node.kind === 'callback' || node.kind === 'module').every(node => node.visibility === undefined));
  assert.ok(graph.nodes.every(node => node.container_visibility === undefined));
  assert.ok(graph.nodes.every(node => node.container === 'src/main.ts'));
});
test('compiler bindings retain anonymous callback ownership and overload implementations', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.ts': `function leaf() {}\nfunction f(x:string):void; function f(x:number):void; function f(x:unknown){leaf()}\nexport function start(){f(1); [1].map(()=>leaf())}\nexport const detached=()=>leaf();` });
  const g = run(root);
  assert.ok(edges(g, 'start').includes('f'));
  assert.ok(edges(g, 'f').includes('leaf'));
  assert.ok(!edges(g, 'start').includes('leaf'));
  assert.ok(g.nodes.some(n => n.kind === 'callback' && edges(g, n.label).includes('leaf')));
});
test('React rules distinguish references, effects, memo and returned callbacks', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.tsx': `import React,{useEffect as effect,useCallback,useMemo,useState,memo} from 'react';\nfunction leaf(){}\nconst Child=memo(function Child({onGo}:{onGo:()=>void}){return <button onClick={onGo}/>});\nexport function Page(){const cb=useCallback(()=>leaf(),[]);effect(()=>{leaf();return ()=>leaf()},[]);useMemo(()=>leaf(),[]);useState(()=>1);return <Child onGo={cb}/>}` });
  const g = run(root);
  assert.ok(g.edges.some(e => e.kind === 'react-render'));
  assert.ok(g.edges.some(e => e.kind === 'react-effect'));
  assert.ok(g.edges.some(e => e.kind === 'react-cleanup'));
  assert.ok(g.edges.some(e => e.kind === 'react-memo'));
  assert.ok(g.edges.some(e => e.kind === 'react-event'));
  assert.ok(!g.edges.some(e => e.kind === 'call' && g.nodes[e.to].label.startsWith('<callback')));
});
test('Next App Router exports are framework entries, not executable starts', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'package.json': { dependencies: { next: '16.4.2', react: '19.3.1' } }, 'src/app/page.tsx': 'export default function Page(){return <div/>}', 'src/app/api/a/route.ts': 'export async function GET(){return new Response("ok")}', 'src/ordinary.ts': 'export function GET(){}' });
  const g = run(root);
  assert.deepEqual(g.entry_points.application_starts, []);
  assert.deepEqual(g.entry_points.framework_entries.map(id=>g.nodes.find(n=>n.id===id).label).sort(), ['GET','Page']);
});
test('project references analyze source without emitted outputs', t => {
  const root = fixture(t, { 'tsconfig.json': { files: [], references: [{path:'./app'}] }, 'lib/tsconfig.json': {compilerOptions:{composite:true, declaration:true,target:'ES2023',module:'esnext',moduleResolution:'bundler',skipLibCheck:true,types:[]},include:['*.ts']}, 'lib/lib.ts':'export function leaf(){}', 'app/tsconfig.json':{compilerOptions:{composite:true,target:'ES2023',module:'esnext',moduleResolution:'bundler',skipLibCheck:true,types:[]},references:[{path:'../lib'}],include:['*.ts']}, 'app/main.ts':'import {leaf} from "../lib/lib"; export function start(){leaf()}' });
  const g = run(root);
  assert.ok(edges(g,'start').includes('leaf'));
  assert.equal(g.nodes.filter(n=>n.label==='leaf').length,1);
  assert.equal(fs.existsSync(path.join(root,'lib/lib.d.ts')),false);
});
test('type errors reject analysis and config preparation reports engine inputs', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.ts':'const value:number="wrong"; export function start(){}' });
  assert.throws(()=>run(root));
  const metadata=run(root,'tsconfig.json','prepare');
  assert.ok(metadata.inputs.some(p=>p.endsWith('typescript.js')));
  assert.match(metadata.versions.typescript,/^6\.0\./);
  assert.equal(metadata.versions.adapter,'5');
});

test('shadowed hook names and cached callbacks never invent execution', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.tsx': `function leaf(){} function useEffect(cb:()=>void){} export function Page(){useEffect(()=>leaf());return <div/>}` });
  const g=run(root);
  assert.equal(g.edges.some(e=>e.kind==='react-effect'),false);
  assert.equal(edges(g,'Page').includes('leaf'),false);
});
test('createElement propagates direct callback props and concise cleanup', t => {
  const root = fixture(t, { 'tsconfig.json': config, 'src/main.tsx': `import {createElement,useEffect} from 'react'; function leaf(){} function Child(props:{onGo:()=>void}){return <button onClick={props.onGo}/>}; export function Page(){useEffect(()=>()=>leaf(),[]);return createElement(Child,{onGo:leaf})}` });
  const g=run(root);
  assert.ok(edges(g,'Child','react-event').includes('leaf'));
  assert.ok(g.edges.some(e=>e.kind==='react-cleanup'));
});
test('distinct configurations claiming one source fail before extraction', t => {
  const root=fixture(t, {'tsconfig.json':{files:[],references:[{path:'a.json'},{path:'b.json'}]},'a.json':{...config,include:['src/*.ts']},'b.json':{...config,include:['src/*.ts']},'src/main.ts':'export function leaf(){}'});
  assert.throws(()=>run(root,'tsconfig.json','prepare'), /Conflicting TypeScript ownership/);
});
test('aliased imports, constructors, accessors and tags retain compiler binding', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/lib.ts':'export function leaf(){} export function tag(t:TemplateStringsArray){leaf()} export class Worker{constructor(){leaf()} get value(){leaf();return 1}}','src/main.ts':'import {leaf as renamed,tag,Worker} from "./lib"; export function start(){renamed();const worker=new Worker();worker.value;tag`hello`;}'});
  const g=run(root);
  assert.ok(edges(g,'start').includes('leaf'));
  assert.ok(edges(g,'start','construct').includes('Worker'));
  assert.ok(edges(g,'start','accessor').includes('value'));
  assert.ok(edges(g,'start').includes('tag'));
});
test('Next utility exports, private routes, unsupported extensions and inactive src/app are not entries', t => {
  const root=fixture(t, {'tsconfig.json':{...config,include:['src/**/*','app/**/*']},'app/page.tsx':'export default function Root(){return <div/>}', 'src/app/page.tsx':'export default function Hidden(){return <div/>}', 'src/app/util.ts':'export function generateMetadata(){}','app/_private/page.tsx':'export default function Private(){return <div/>}','app/api/route.mts':'export function GET(){}'});
  const g=run(root);
  assert.deepEqual(g.entry_points.framework_entries.map(id=>g.nodes.find(n=>n.id===id).label),['Root']);
});
test('propagated callback invocations are possible, ordinary direct bindings stay direct', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.tsx':'function leaf(){} function Child({onGo}:{onGo:()=>void}){onGo();(onGo)();return <div/>} export function Page(){leaf();return <Child onGo={leaf}/>}'});
  const g=run(root);
  const e=g.edges.find(e=>e.kind==='call'&&g.nodes[e.from].label==='Child'&&g.nodes[e.to].label==='leaf');
  assert.equal(e.certainty,'possible');
  assert.ok(g.edges.filter(edge=>g.nodes[edge.from].label==='Child' && edge.kind==='call').every(edge=>edge.certainty==='possible'));
  assert.equal(g.edges.find(e=>e.kind==='call'&&g.nodes[e.from].label==='Page'&&g.nodes[e.to].label==='leaf').certainty,'direct');
});

test('instance and static field initializer scopes are excluded independently of module execution', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.ts':'function leaf(){} class Explicit{x=leaf(); constructor(){}} class Implicit{x=leaf();static y=leaf()} export function start(){new Explicit();new Implicit()}'});
  const g=run(root);
  const removed=new Set(g.excluded_methods);
  assert.equal(g.nodes.filter(n=>n.kind==='initializer').length,3);
  assert.ok(g.nodes.filter(n=>n.kind==='initializer').every(n=>removed.has(n.id)&&edges(g,n.label).includes('leaf')));
  assert.equal(g.nodes.filter(n=>n.kind==='module').some(n=>edges(g,n.label).includes('leaf')),false);
});

test('conditional function values preserve possible targets instead of arbitrary signature origins', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.ts':'declare const choose:boolean; function a(){} function b(){} const f=choose?a:b; export function entry(){f()}' });
  const g=run(root);
  const calls=g.edges.filter(e=>g.nodes[e.from].label==='entry');
  assert.deepEqual(calls.map(e=>g.nodes[e.to].label).sort(),['a','b']);
  assert.ok(calls.every(e=>e.certainty==='possible'));
});
test('bracket access and updates call the correct getter/setter', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.ts':'class A{ get value(){return 1} set value(v:number){} } export function entry(){const a=new A();a["value"];a["value"]=2;a["value"]++;a.value+=2;}' });
  const g=run(root);
  const calls=g.edges.filter(e=>e.kind==='accessor');
  assert.equal(calls.length,2);
  assert.deepEqual(calls.map(e=>e.count).sort(),[3,3]);
});
test('reducer initialization, forwardRef and returned useCallback functions retain static bindings', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.tsx':`import {useReducer,useCallback,forwardRef} from 'react'; function leaf(){} function reducer(n:number,a:unknown){leaf();return n} function init(n:number){leaf();return n} const Child=forwardRef<HTMLDivElement>(function Child(p,r){return <div ref={r}/>}); export function Page(){useReducer(reducer,0,init);const callback=useCallback(leaf,[]);callback();return <Child/>}`});
  const g=run(root);
  assert.ok(edges(g,'Page','react-reducer').includes('reducer'));
  assert.ok(edges(g,'Page','react-initializer').includes('init'));
  assert.ok(edges(g,'Page','react-render').includes('Child'));
  assert.ok(edges(g,'Page','call').includes('leaf'));
});
test('Next page exports and module/inline server actions are entries', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/app/page.tsx':`export default function Page(){return <div/>} export function generateMetadata(){} export function generateStaticParams(){} export async function save(){'use server';}`,'src/actions.ts':`'use server'; export async function action(){}`,'src/app/util.ts':'export function generateMetadata(){}'});
  const g=run(root);
  const entries=g.entry_points.framework_entries.map(id=>g.nodes.find(n=>n.id===id));
  assert.deepEqual(entries.map(n=>n.label).sort(),['Page','action','generateMetadata','generateStaticParams','save']);
  assert.ok(!entries.some(n=>n.file.endsWith('/util.ts')));
});
test('config inheritance, path aliases and empty nested discovery directories are captured', t => {
  const root=fixture(t, {'base.json':{compilerOptions:{...config.compilerOptions,paths:{'@/*':['./src/*']}}},'tsconfig.json':{extends:'./base.json',include:['src/**/*']},'src/leaf.ts':'export function leaf(){}','src/main.ts':'import {leaf} from "@/leaf";export function start(){leaf()}'});
  fs.mkdirSync(path.join(root,'src/empty/nested'),{recursive:true});
  const metadata=run(root,'tsconfig.json','prepare');
  assert.ok(metadata.inputs.some(p=>p.endsWith('/base.json')));
  assert.ok(metadata.watch_roots.some(p=>p.endsWith('/src/empty/nested')));
  assert.ok(metadata.output_roots.some(p=>p.endsWith('/.next')));
  assert.ok(edges(run(root),'start').includes('leaf'));
});
test('missing generated include directories remain watched inside excluded output parents', t => {
  const root=fixture(t, {'tsconfig.json':{...config,include:['src/**/*','.next/types/**/*.ts']},'src/main.ts':'export function start(){}'});
  const generated=path.join(fs.realpathSync(root),'.next/types');
  const before=run(root,'tsconfig.json','prepare');
  assert.ok(before.watch_roots.includes(generated),'absent explicit wildcard root must retain its missing fingerprint');
  fs.mkdirSync(generated,{recursive:true});
  fs.writeFileSync(path.join(generated,'entry.ts'),'export function generatedEntry(){}');
  const after=run(root,'tsconfig.json','prepare');
  assert.ok(after.watch_roots.includes(generated));
  assert.ok(after.sources.some(source=>source.endsWith('/.next/types/entry.ts')));
});
test('explicit source directories inside compiler output roots retain membership watches', t => {
  const root=fixture(t, {'tsconfig.json':{compilerOptions:{...config.compilerOptions,outDir:'dist'},files:['src/main.ts','dist/types/entry.ts']},'src/main.ts':'export function start(){}','dist/types/entry.ts':'export function generatedEntry(){}'});
  fs.mkdirSync(path.join(root,'dist/types/empty'),{recursive:true});
  const metadata=run(root,'tsconfig.json','prepare');
  assert.ok(metadata.watch_roots.includes(path.join(fs.realpathSync(root),'dist/types')));
  assert.ok(metadata.watch_roots.includes(path.join(fs.realpathSync(root),'dist/types/empty')));
});

test('source policy marks boilerplate without removing overloads or authored callables', t => {
  const root=fixture(t, {'tsconfig.json':config,'src/main.ts':`function leaf(){} function toString(){leaf()}
class Base { constructor(){leaf()} toString(){return ''} valueOf(){return this} hasOwnProperty(v:PropertyKey){return false} }
class Other { toString(v:string){return v} private hidden(){leaf()} protected guarded(){leaf()} public shown(){leaf()} get value(){leaf();return 1} }
class Implicit { x=leaf(); static y=leaf(); static { leaf() } callback=()=>leaf() }
export function start(){toString();new Base();new Implicit();new Other().toString('x');}`});
  const g=run(root);
  const removed=new Set(g.excluded_methods);
  const excluded=g.nodes.filter(n=>removed.has(n.id));
  assert.ok(excluded.some(n=>n.kind==='constructor'));
  assert.ok(excluded.some(n=>n.kind==='implicit-constructor'));
  assert.ok(excluded.some(n=>n.kind==='initializer'));
  assert.equal(g.nodes.filter(n=>n.label==='toString'&&removed.has(n.id)).length,1);
  assert.ok(['hidden','guarded','shown','value','callback'].every(label=>g.nodes.some(n=>n.label===label&&!removed.has(n.id))));
  assert.equal(g.nodes.filter(n=>n.kind==='module').flatMap(n=>edges(g,n.label)).includes('leaf'),false);
  assert.equal(edges(g,'start').includes('leaf'),false);
});

test('architecture facts retain type-only files, decorators and inherited base types', t => {
  const source = `function Layer(...args:any[]):any { return undefined; }
class Grand {}
@Layer class Base extends Grand {}
@Layer export class Controller extends Base { @Layer run() {} }
export interface Contract { run():void }
export type Result = string;`;
  const root = fixture(t, { 'tsconfig.json': {...config, compilerOptions:{...config.compilerOptions,experimentalDecorators:true}}, 'src/main.ts':source, 'src/types.ts':'export interface Data { value: string }' });
  const graph = run(root);
  assert.equal(graph.architecture.version,1);
  const facts=graph.architecture.files.find(f=>f.file.endsWith('/src/main.ts'));
  assert.ok(facts.names.includes('Controller'));
  assert.ok(facts.kinds.includes('interface'));
  assert.ok(facts.annotations.includes('Layer'));
  assert.ok(facts.method_annotations.includes('Layer'));
  assert.ok(facts.base_types.includes('Base'));
  assert.ok(facts.base_types.includes('Grand'));
  assert.equal(facts.content,source);
  assert.ok(graph.architecture.files.some(f=>f.file.endsWith('/src/types.ts')&&f.names.includes('Data')));
});

test('default class method export identities survive body edits and method movement',t=>{
  const source=`export default class {first(){return 1}second(){return 2}}
    export const Assigned=class {first(){return 1}};
    export const holder={value:class {first(){return 1}}};
    export function siblings(){return [class {first(){return 1}},class {first(){return 1}}]}`;
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  fs.writeFileSync(path.join(root,'src/main.ts'),source.replace('first(){return 1}second(){return 2}','second(){return 2}first(){/* body edit */return 3}'));
  const after=run(root);
  const keys=graph=>graph.nodes.filter(node=>['first','second'].includes(node.label)).map(node=>graph.export_identities[node.id]).sort();
  assert.equal(keys(before).length,6);
  assert.equal(new Set(keys(before)).size,6,'Default, assigned, property and anonymous sibling class owners remain distinct');
  assert.deepEqual(keys(after),keys(before));
});

test('anonymous regex bodies keep distinct fingerprints when callbacks move',t=>{
  const first="()=>/[/*]a/.test('a')",second="()=>/[/*]b/.test('b')";
  const source=`export function callbacks(){return [${first},${second}]}`;
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  const moved=`export function callbacks(){return [${second},${first}]}`;
  fs.writeFileSync(path.join(root,'src/main.ts'),moved);
  const after=run(root);
  const keys=(graph,text)=>Object.fromEntries(graph.nodes.filter(node=>node.kind==='callback').map(node=>{
    const flow=graph.flow_inventory.functions.find(flow=>flow.symbol===node.id);
    return [text.slice(flow.span.start,flow.span.end),graph.export_identities[node.id]];
  }));
  assert.deepEqual(keys(before,source),keys(after,moved));
});

test('source aliases use canonical display facts while retaining native compiler identities',t=>{
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':'export function leaf(){return 1} export function entry(){return leaf()}'});
  const alias=`${root}-alias`;
  fs.symlinkSync(root,alias,'dir');t.after(()=>fs.rmSync(alias,{force:true}));
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({...config,include:undefined,files:[path.join(alias,'src/main.ts')]}));
  const graph=run(root),leaf=graph.nodes.find(node=>node.label==='leaf');
  assert.equal(leaf.file,fs.realpathSync(path.join(root,'src/main.ts')));
  assert.ok(leaf.id.includes(`${alias}/src/main.ts|`),'Native binding identities retain the compiler spelling');
  assert.deepEqual(edges(graph,'entry'),['leaf']);
  assert.ok(graph.architecture.files.every(file=>file.file===leaf.file));
  assert.ok(graph.flow_inventory.functions.every(flow=>flow.file===leaf.file));
});

test('computed and quoted declaration names ignore spelling trivia in export identities',t=>{
  const source=`export class Sequence{[Symbol.iterator](){return [1].values()} 'quoted'(){return 1}}
    export const holder={'quoted'(){return 2},[Symbol.toStringTag](){return 'value'},['literal'](){return 3}};`;
  const root=fixture(t,{'tsconfig.json':config,'src/main.ts':source});
  const before=run(root);
  const changed=source.replace('[Symbol.iterator]','[ /* comment */ Symbol . iterator ]')
    .replace('[Symbol.toStringTag]','[Symbol /* tag */ . toStringTag]').replaceAll("'quoted'",'"quoted"').replace("['literal']",'["literal"]');
  fs.writeFileSync(path.join(root,'src/main.ts'),changed);
  const after=run(root);
  const keys=graph=>graph.nodes.filter(node=>node.kind==='function').map(node=>graph.export_identities[node.id]).sort();
  assert.equal(keys(before).length,5);
  assert.equal(new Set(keys(before)).size,5);
  assert.deepEqual(keys(after),keys(before));
});
