const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const cp = require('node:child_process');
const engineModules = process.env.CODEGRAPH_TS_NODE_MODULES || (process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname, '../../../PacificFateMapEditor/node_modules');
function fixture(t, files) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'codegraph-ts-'));
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
function edges(graph, label, kind) {
  return graph.edges.filter(e => graph.nodes[e.from].label === label && (!kind || e.kind === kind)).map(e => graph.nodes[e.to].label);
}
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
