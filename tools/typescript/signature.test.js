'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const os=require('node:os');
const cp=require('node:child_process');

const {relativeImportTypes}=require('./signature.js');
const modules=process.env.CODECALLNAVIGATOR_TS_NODE_MODULES||path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
const ts=require(path.join(modules,'typescript'));

function analyze(root,literal,linkedSource) {
  fs.mkdirSync(root,{recursive:true});
  fs.symlinkSync(modules,path.join(root,'node_modules'),'dir');
  const include=linkedSource?'src/*.ts':'*.ts';
  fs.writeFileSync(path.join(root,'tsconfig.json'),JSON.stringify({compilerOptions:{target:'ES2023',module:'esnext',moduleResolution:'bundler',types:[]},include:[include]}));
  if(linkedSource) fs.symlinkSync(linkedSource,path.join(root,'src'),'dir');
  else writeSources(root,literal);
  const output=path.join(root,'result.json');
  cp.execFileSync(process.execPath,[path.join(__dirname,'analyze.js'),'analyze',path.join(root,'tsconfig.json'),output],{stdio:'pipe'});
  return JSON.parse(fs.readFileSync(output));
}
function writeSources(root,literal) {
  const patterned='export function patterned<T extends string>(value: `${T}import('+JSON.stringify(literal)+')`) {return import("./other")}';
  fs.writeFileSync(path.join(root,'app.ts'),[
    `export function f(value: ${JSON.stringify(literal)}) {return value}`,
    "export function make(){return import('./other')}",patterned].join('\n'));
  fs.writeFileSync(path.join(root,'other.ts'),'export class Widget {value(){return 1}}');
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
  assert.deepEqual(callable(first,'patterned'),callable(second,'patterned'));
  assert.ok(callable(first,'patterned').signature.includes(literal));
  assert.ok(callable(first,'patterned').signature.includes('import("./other")'));
  const linked=analyze(path.join(root,'linked'),literal,path.join(root,'first'));
  for(const name of ['f','make','patterned']) assert.deepEqual(callable(first,name),callable(linked,name));
});

// Use a private physical root so canonicalization never depends on a machine's /project.
function scannerFixture(t) {
  const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-signature-policy-'));
  t.after(()=>fs.rmSync(temporary,{recursive:true,force:true}));
  const root=fs.realpathSync(temporary);
  return signature=>relativeImportTypes(ts,signature.replaceAll('/project',root),path.join(root,'src/app.ts')).replaceAll(root,'/project');
}

test('signature scanner rewrites only absolute import-type module arguments',t=>{
  const normalize=scannerFixture(t);
  const signature='(value: "/project/src/literal", text: \'import("/project/src/literal")\'): Promise<typeof import("/project/src/other")>';
  const expected='(value: "/project/src/literal", text: \'import("/project/src/literal")\'): Promise<typeof import("./other")>';
  assert.equal(normalize(signature),expected);
  assert.equal(normalize('(): import("./other").Widget'),'(): import("./other").Widget');
});
test('signature scanner preserves nested template text and interpolation braces',t=>{
  const normalize=scannerFixture(t);
  const signature='(value: `${T extends {key: string} ? `${T & string}import("/project/src/literal")` : string}import("/project/src/tail")}`): import("/project/types/other").Widget';
  const expected='(value: `${T extends {key: string} ? `${T & string}import("/project/src/literal")` : string}import("/project/src/tail")}`): import("../types/other").Widget';
  assert.equal(normalize(signature),expected);
});
test('signature scanner retains truncation and incomplete module literals',t=>{
  const normalize=scannerFixture(t);
  const signature='(): Promise<typeof import("/project/src/other")> & { ... 42 more ... }';
  assert.equal(normalize(signature),'(): Promise<typeof import("./other")> & { ... 42 more ... }');
  const incomplete='(): import("/project/src/incomplete';
  assert.equal(normalize(incomplete),incomplete);
});
