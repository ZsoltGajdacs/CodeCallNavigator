'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const cp=require('node:child_process');

function analyze(root,literal) {
  fs.mkdirSync(root,{recursive:true});
  fs.symlinkSync(process.env.CODECALLNAVIGATOR_TS_NODE_MODULES||path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules'),path.join(root,'node_modules'),'dir');
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',types:[]},include:['*.ts']}));
  fs.writeFileSync(path.join(root,'app.ts'),`export function f(value: ${JSON.stringify(literal)}) {return value}
    export function make(){return import('./other')}`);
  fs.writeFileSync(path.join(root,'other.ts'),'export class Widget {value(){return 1}}');
  const output=path.join(root,'result.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),output],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(output));
}
function callable(graph,name) {
  const node=graph.nodes.find(node=>node.label===name);
  assert.ok(node,`Missing ${name}`);
  return {signature:node.signature,semantic:graph.export_identities[node.id]};
}
test('contextual compiler signatures keep import types relative and source literal types unchanged',t=>{
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-signature-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  const literal=path.join(fs.realpathSync(root),'first','literal');
  const first=analyze(path.join(root,'first'),literal);
  const second=analyze(path.join(root,'second'),literal);
  assert.deepEqual(callable(first,'f'),callable(second,'f'));
  assert.ok(callable(first,'f').signature.includes(literal));
  assert.deepEqual(callable(first,'make'),callable(second,'make'));
  assert.ok(callable(first,'make').signature.includes('import("./other")'));
  assert.ok(!callable(first,'make').signature.includes(root));
});
