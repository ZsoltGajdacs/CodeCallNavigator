'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {spawn}=require('node:child_process');
const {setTimeout:delay}=require('node:timers/promises');
const engineModules=process.env.CODEGRAPH_TS_NODE_MODULES || (process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODEGRAPH_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
async function waitFor(predicate,process) {
  const deadline=Date.now()+30000;
  while (!predicate()) {
    if (process.exitCode!==null || process.signalCode!==null) throw new Error('Worker exited before metadata');
    if (Date.now()>deadline) throw new Error('Worker metadata timed out');
    await delay(10);
  }
}
async function fixture(t,source='export function leaf(){return 1;}') {
  const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'codegraph-worker-'));
  const root=path.join(temporary,'project'); fs.mkdirSync(root);
  fs.mkdirSync(path.join(root,'src'));
  fs.symlinkSync(engineModules,path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{strict:true,skipLibCheck:true,target:'ES2023',module:'esnext',moduleResolution:'bundler',types:[]},include:['src/**/*.ts']}));
  const file=path.join(root,'src/main.ts'); fs.writeFileSync(file,source);
  const metadata=path.join(temporary,'metadata.json'), output=path.join(temporary,'graph.json');
  const child=spawn(process.execPath,[path.join(__dirname,'analyze.js'),'worker',path.join(root,'tsconfig.json'),metadata],{stdio:['pipe','pipe','pipe']});
  let errors=''; child.stderr.on('data',data=>{errors+=data;});
  const exited=new Promise(resolve=>child.on('exit',(code,signal)=>resolve({code,signal})));
  t.after(async()=>{if(child.exitCode===null && child.signalCode===null) {child.kill();await exited;} fs.rmSync(temporary,{recursive:true,force:true});});
  await waitFor(()=>fs.existsSync(metadata),child);
  return {root,file,output,metadata,child,errors:()=>errors,exited};
}
async function analyze(worker) {
  worker.child.stdin.end(JSON.stringify({command:'analyze',output:worker.output})+'\n');
  return worker.exited;
}
test('worker retains compiler output and exits after extraction',async t=>{
  const worker=await fixture(t,'export function leaf(){} export function start(){leaf()}');
  assert.equal((await analyze(worker)).code,0,worker.errors());
  const graph=JSON.parse(fs.readFileSync(worker.output,'utf8'));
  assert.ok(graph.edges.some(edge=>graph.nodes[edge.from].label==='start' && graph.nodes[edge.to].label==='leaf'));
  assert.equal(fs.existsSync(path.join(worker.root,'src/main.js')),false);
});
test('worker rejects retained source changes with restored size and timestamp',async t=>{
  const worker=await fixture(t), before=fs.statSync(worker.file);
  fs.writeFileSync(worker.file,'export function leaf(){return 2;}');
  fs.utimesSync(worker.file,before.atime,before.mtime);
  assert.notEqual((await analyze(worker)).code,0);
  assert.match(worker.errors(),/changed/);
  assert.equal(fs.existsSync(worker.output),false);
});
test('worker rejects source membership added after preparation',async t=>{
  const worker=await fixture(t);
  fs.writeFileSync(path.join(worker.root,'src/new.ts'),'export function added(){}');
  assert.notEqual((await analyze(worker)).code,0);
  assert.match(worker.errors(),/changed/);
  assert.equal(fs.existsSync(worker.output),false);
});
test('worker rejects newly appearing module-resolution candidates',async t=>{
  const worker=await fixture(t,'// @ts-ignore\nimport {leaf} from "./missing"; export function start(){leaf()}');
  fs.writeFileSync(path.join(worker.root,'src/missing.ts'),'export function leaf(){}');
  assert.notEqual((await analyze(worker)).code,0);
  assert.match(worker.errors(),/changed/);
});
test('worker rejects invalid requests without graph output',async t=>{
  const worker=await fixture(t);
  worker.child.stdin.end('{"command":"other"}\n');
  assert.notEqual((await worker.exited).code,0);
  assert.match(worker.errors(),/Invalid TypeScript worker request/);
  assert.equal(fs.existsSync(worker.output),false);
});
