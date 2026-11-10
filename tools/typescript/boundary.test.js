'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const cp=require('node:child_process');
function analyze(t,files) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codegraph-boundary-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  fs.symlinkSync(process.env.CODEGRAPH_TS_NODE_MODULES||path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules'),path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',jsx:'react-jsx',strict:true,skipLibCheck:true},include:['**/*.ts','**/*.tsx']}));
  for(const [file,text] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(root,file)),{recursive:true});
    fs.writeFileSync(path.join(root,file),text);
  }
  const out=path.join(root,'result.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),out],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(out));
}
test('bound Next APIs and builtin fetch produce exact source facts without synchronous edges',t=>{
  const graph=analyze(t,{
    'app/about/page.tsx':'export default function About(){return <main/>}',
    'app/users/[id]/page.tsx':'export default function User(){return <main/>}',
    'app/api/route.ts':'export function GET(){return new Response("ok")}',
    'app/page.tsx':`import Link from 'next/link';import {useRouter,redirect} from 'next/navigation';
      export default function Page(){const router=useRouter();function go(){router.push('/about');router.replace('/users/42');router.prefetch('/about')}
      return <Link href="/about">About</Link>}
      export function request(){return fetch('/api',{method:'GET'})}
      export function leave(){redirect('/about')}`,
  });
  assert.equal(graph.boundaries.version,1);
  assert.deepEqual(graph.boundaries.routes.map(route=>[route.kind,route.path]).sort(),[
    ['http','/api'],['navigation','/'],['navigation','/about'],['navigation','/users/{id}']].sort());
  assert.equal(graph.boundaries.uses.length,5);
  assert.equal(graph.boundaries.uses.filter(use=>use.flow).length,4);
  for(const use of graph.boundaries.uses) assert.ok(graph.nodes.some(node=>node.id===use.symbol));
  assert.ok(!graph.edges.some(edge=>['navigation','http'].includes(edge.kind)));
});
test('shadowed APIs, changed routers, dynamic URLs and unknown fetch options stay unresolved',t=>{
  const graph=analyze(t,{'app/page.tsx':`import {useRouter} from 'next/navigation';
    export default function Page(){let router=useRouter();router={push() {}} as any;router.push('/about');return null}
    export function fake(){const fetch=(path:string)=>path;return fetch('/api')}
    export function dynamic(path:string){return fetch(path)}
    export function options(init:RequestInit){return fetch('/api',init)}
    export function external(){return fetch('https://example.invalid/api')}`});
  assert.deepEqual(graph.boundaries.uses,[]);
});
test('route groups preserve ambiguity and catchalls remain blockers',t=>{
  const graph=analyze(t,{
    'app/(one)/same/page.tsx':'export default function One(){return null}',
    'app/(two)/same/page.tsx':'export default function Two(){return null}',
    'app/[...rest]/page.tsx':'export default function Catch(){return null}',
  });
  assert.equal(graph.boundaries.routes.filter(route=>route.path==='/same').length,2);
  assert.ok(graph.boundaries.routes.some(route=>route.restrictions.includes('unsupported-route-segment')));
});
test('global fetch replacement and router property replacement do not establish boundaries',t=>{
  const graph=analyze(t,{'app/page.tsx':`import {useRouter} from 'next/navigation';
    globalThis.fetch=async()=>new Response('replacement');
    export function request(){return fetch('/api')}
    export default function Page(){const router=useRouter();router.push=()=>{};router.push('/about');return null}`});
  assert.deepEqual(graph.boundaries.uses,[]);
});
test('Pages Router useRouter is outside App Router inference',t=>{
  const graph=analyze(t,{'app/page.tsx':`import {useRouter} from 'next/router';
    export default function Page(){const router=useRouter();router.push('/about');return null}`});
  assert.deepEqual(graph.boundaries.uses,[]);
});
test('Link display alias is deferred and implicit HEAD stays attached to GET',t=>{
  const graph=analyze(t,{
    'app/api/route.ts':"export function GET(){return new Response('ok')}",
    'app/page.tsx':`import Link from 'next/link';export default function Page(){return <Link href="/about" as="/other">Other</Link>}`,
  });
  assert.deepEqual(graph.boundaries.uses,[]);
  assert.deepEqual(graph.boundaries.routes.find(route=>route.kind==='http').methods,['GET','HEAD']);
});
test('structurally typed global object writes and escapes invalidate builtin fetch inference',t=>{
  for(const replacement of [
    `(globalThis as unknown as {fetch: typeof fetch}).fetch=async()=>new Response('replaced')`,
    `const env:{fetch:typeof fetch}=globalThis;env.fetch=async()=>new Response('replaced')`,
    `const env:{fetch:typeof fetch}=window;env.fetch=async()=>new Response('replaced')`,
    `const env:{fetch:typeof fetch}=self;env.fetch=async()=>new Response('replaced')`,
    `window.window.fetch=async()=>new Response('replaced')`,
  ]) {
    const graph=analyze(t,{'main.ts':`${replacement};export function request(){return fetch('/api')}`});
    assert.deepEqual(graph.boundaries.uses,[]);
  }
});
test('deferred class field initializers are not attributed to the enclosing factory',t=>{
  const graph=analyze(t,{'main.ts':`export function factory(){return class {
    value=fetch('/api');static other=fetch('/api');static {fetch('/api')}
    method(){return fetch('/api')}
  }}`});
  assert.equal(graph.boundaries.uses.length,1);
  const owner=graph.nodes.find(node=>node.id===graph.boundaries.uses[0].symbol);
  assert.equal(owner.label,'method');
});
