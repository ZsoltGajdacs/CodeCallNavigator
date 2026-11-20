'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {spawn}=require('node:child_process');
const {setTimeout:delay}=require('node:timers/promises');
const engineModules=process.env.CODECALLNAVIGATOR_TS_NODE_MODULES || (process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT)) || path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
async function waitFor(predicate,process) {
  const deadline=Date.now()+30000;
  while (!predicate()) {
    if (process.exitCode!==null || process.signalCode!==null) throw new Error('Worker exited before metadata');
    if (Date.now()>deadline) throw new Error('Worker metadata timed out');
    await delay(10);
  }
}
async function fixture(t,source='export function leaf(){return 1;}',copyAdapter=false) {
  const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-worker-'));
  const root=path.join(temporary,'project'); fs.mkdirSync(root);
  fs.mkdirSync(path.join(root,'src'));
  fs.symlinkSync(engineModules,path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{strict:true,skipLibCheck:true,target:'ES2023',module:'esnext',moduleResolution:'bundler',types:[]},include:['src/**/*.ts']}));
  const file=path.join(root,'src/main.ts'); fs.writeFileSync(file,source);
  const response=path.join(temporary,'response.json');
  const adapter=copyAdapter?path.join(temporary,'adapter'):__dirname;
  if(copyAdapter) fs.cpSync(__dirname,adapter,{recursive:true});
  const child=spawn(process.execPath,[path.join(adapter,'analyze.js'),'worker',path.join(root,'tsconfig.json'),response],{stdio:['pipe','pipe','pipe']});
  let errors=''; child.stderr.on('data',data=>{errors+=data;});
  const exited=new Promise(resolve=>child.on('exit',(code,signal)=>resolve({code,signal})));
  t.after(async()=>{if(child.exitCode===null && child.signalCode===null) {child.kill();await exited;} fs.rmSync(temporary,{recursive:true,force:true});});
  const worker={root,file,response,child,adapter,errors:()=>errors,exited};
  const initial=await reply(worker,0);
  assert.equal(initial.status,'success',initial.error?.message);
  return {...worker,metadata:initial.metadata};
}
async function reply(worker,sequence) {
  let result;
  await waitFor(()=>{
    if (!fs.existsSync(worker.response)) return false;
    result=JSON.parse(fs.readFileSync(worker.response,'utf8'));
    return result.sequence===sequence;
  },worker.child);
  assert.equal(result.version,1);
  return result;
}
async function request(worker,command,sequence=1) {
  worker.child.stdin.write(JSON.stringify({version:1,sequence,command})+'\n');
  return reply(worker,sequence);
}
function changed(result,file) {
  assert.equal(result.status,'error');
  assert.equal(result.error.code,'inputs_changed');
  assert.match(result.error.message,/changed/);
  if (file) assert.equal(result.error.path,file);
  assert.equal(result.graph,undefined);
}
test('worker retains compiler output and exits after extraction',async t=>{
  const worker=await fixture(t,'export function leaf(){} export function start(){leaf()}');
  const incidental=path.join(worker.root,'src/generated-artifact');
  fs.mkdirSync(incidental);
  fs.writeFileSync(path.join(incidental,'bundle.css'),'.sample { display: block; }');
  assert.equal(worker.metadata.versions.build_evidence,'1');
  assert.ok(worker.metadata.dependency_inputs.some(file=>file.endsWith('typescript.js')));
  const result=await request(worker,'analyze');
  assert.equal(result.status,'success',result.error?.message);
  const graph=result.graph;
  assert.ok(graph.edges.some(edge=>graph.nodes[edge.from].label==='start' && graph.nodes[edge.to].label==='leaf'));
  assert.equal(fs.existsSync(path.join(worker.root,'src/main.js')),false);
  worker.child.stdin.end();
  assert.equal((await worker.exited).code,0,worker.errors());
});
test('worker rejects retained source changes with restored size and timestamp',async t=>{
  const worker=await fixture(t), before=fs.statSync(worker.file);
  fs.writeFileSync(worker.file,'export function leaf(){return 2;}');
  fs.utimesSync(worker.file,before.atime,before.mtime);
  changed(await request(worker,'analyze'),fs.realpathSync(worker.file));
  const update=await request(worker,'update',2);
  assert.equal(update.status,'success',update.error?.message);
  assert.ok(update.affected>0);
  assert.equal((await request(worker,'analyze',3)).status,'success');
});
test('worker rejects source membership added after preparation',async t=>{
  const worker=await fixture(t);
  fs.writeFileSync(path.join(worker.root,'src/new.ts'),'export function added(){}');
  changed(await request(worker,'analyze'));
});
test('worker rejects newly appearing module-resolution candidates',async t=>{
  const worker=await fixture(t,'// @ts-ignore\nimport {leaf} from "./missing"; export function start(){leaf()}');
  assert.equal((await request(worker,'verify')).status,'success');
  fs.writeFileSync(path.join(worker.root,'src/missing.ts'),'export function leaf(){}');
  changed(await request(worker,'verify',2));
  changed(await request(worker,'analyze',3));
});
test('worker rejects invalid requests without graph output',async t=>{
  const worker=await fixture(t);
  worker.child.stdin.end('{"version":1,"sequence":1,"command":"other"}\n');
  assert.notEqual((await worker.exited).code,0);
  assert.match(worker.errors(),/Invalid TypeScript worker request/);
  const result=JSON.parse(fs.readFileSync(worker.response,'utf8'));
  assert.equal(result.status,'error');
  assert.equal(result.graph,undefined);
});

// A real retained compiler worker is required to prove policy edits cannot publish output.
test('worker rejects compatibility policy changes after preparation',async t=>{
  const worker=await fixture(t,'export function leaf(){return 1;}',true);
  const policy=path.join(worker.adapter,'compatibility.json');
  assert.ok(worker.metadata.inputs.includes(fs.realpathSync(policy)));
  fs.appendFileSync(policy,'\n');
  const result=await request(worker,'analyze');
  assert.equal(result.status,'restart');
  assert.equal(result.error.code,'restart_required');
  assert.match(result.error.message,/compatibility policy changed/);
  assert.equal(result.graph,undefined);
});
