'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const {Observations} = require('./observations.js');
function fixture(t) {
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'codegraph-observations-'));
  t.after(()=>fs.rmSync(root,{recursive:true,force:true}));
  return root;
}
test('retained read content rejects same-size edits with restored timestamps',t=>{
  const root=fixture(t), file=path.join(root,'source.ts');
  fs.writeFileSync(file,'export const one=1;');
  const before=fs.statSync(file), observations=new Observations();
  observations.observe('source',()=>fs.readFileSync(file,'utf8'));
  fs.writeFileSync(file,'export const two=2;');
  fs.utimesSync(file,before.atime,before.mtime);
  assert.throws(()=>observations.verify(),/changed/);
});
test('negative resolution probes reject appearing files',t=>{
  const root=fixture(t), file=path.join(root,'missing.ts'), observations=new Observations();
  observations.observe('probe',()=>fs.existsSync(file));
  fs.writeFileSync(file,'export {};');
  assert.throws(()=>observations.verify(),/changed/);
});
test('directory query results reject additions',t=>{
  const root=fixture(t), observations=new Observations();
  observations.observe('directory',()=>fs.readdirSync(root));
  fs.writeFileSync(path.join(root,'added.ts'),'export {};');
  assert.throws(()=>observations.verify(),/changed/);
});
test('different values observed during preparation are rejected immediately',()=>{
  const observations=new Observations();
  observations.observe('input',()=> 'first');
  assert.throws(()=>observations.observe('input',()=> 'later'),/changed/);
});
test('unchanged observations may be queried repeatedly and verified',()=>{
  const observations=new Observations();
  assert.equal(observations.observe('input',()=> 'same'),'same');
  assert.equal(observations.observe('input',()=> 'same'),'same');
  observations.verify();
});
