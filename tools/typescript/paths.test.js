'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const os=require('node:os');
const path=require('node:path');
const {canonical}=require('./paths.js');
const {trackingSystem}=require('./compiler.js');
const {Observations}=require('./observations.js');
const modules=process.env.CODECALLNAVIGATOR_TS_NODE_MODULES ||
  (process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT && path.dirname(process.env.CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT)) ||
  path.resolve(__dirname,'../../../PacificFateMapEditor/node_modules');
const ts=require(path.join(modules,'typescript'));

function fixture(t) {
  const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'codecallnavigator-paths-'));
  t.after(()=>fs.rmSync(temporary,{recursive:true,force:true}));
  const root=fs.realpathSync(temporary);
  const first=path.join(root,'first'), second=path.join(root,'second'), alias=path.join(root,'alias');
  fs.mkdirSync(first);fs.mkdirSync(second);fs.symlinkSync(first,alias,'dir');
  return {first,second,alias};
}

function tracked() {
  const inputs=new Set(), observations=new Observations();
  return {inputs,observations,system:trackingSystem(ts,inputs,observations)};
}

test('existing source paths retain their physical spelling',t=>{
  const {first,alias}=fixture(t);
  fs.writeFileSync(path.join(first,'source.ts'),'export {};');
  assert.equal(canonical(path.join(alias,'source.ts')),path.join(first,'source.ts'));
  assert.equal(canonical(first),first);
});

test('missing descendants resolve through the nearest existing physical ancestor',t=>{
  const {first,alias}=fixture(t);
  assert.equal(canonical(path.join(alias,'missing/deeper/source.ts')),path.join(first,'missing/deeper/source.ts'));
});

test('missing absolute paths terminate at the filesystem root',()=>{
  const root=path.parse(os.tmpdir()).root;
  const missing=path.join(root,`__codecallnavigator_absent_${process.pid}`, 'nested/source.ts');
  assert.equal(fs.existsSync(missing),false);
  assert.equal(canonical(missing),missing);
});

test('normalization reads current symlink targets without a path cache',t=>{
  const {first,second,alias}=fixture(t), probe=path.join(alias,'missing/source.ts');
  assert.equal(canonical(probe),path.join(first,'missing/source.ts'));
  fs.unlinkSync(alias);fs.symlinkSync(second,alias,'dir');
  assert.equal(canonical(probe),path.join(second,'missing/source.ts'));
});

test('compiler input metadata records normalized missing paths',t=>{
  const {first,alias}=fixture(t), {inputs,system}=tracked();
  assert.equal(system.fileExists(path.join(alias,'missing.ts')),false);
  assert.deepEqual([...inputs],[path.join(first,'missing.ts')]);
});

test('original compiler probes reject files appearing after preparation',t=>{
  const {first,alias}=fixture(t), {system,observations}=tracked();
  const probe=path.join(alias,'missing.ts');
  assert.equal(system.fileExists(probe),false);
  fs.writeFileSync(path.join(first,'missing.ts'),'export {};');
  assert.throws(()=>observations.verify(),/changed/);
});

test('observations reject symlink retargeting even when both probes remain missing',t=>{
  const {second,alias}=fixture(t), {system,observations}=tracked();
  const probe=path.join(alias,'missing.ts');
  assert.equal(system.fileExists(probe),false);
  fs.unlinkSync(alias);fs.symlinkSync(second,alias,'dir');
  assert.equal(ts.sys.fileExists(probe),false);
  assert.throws(()=>observations.verify(),/changed/);
});

test('repeated preparation probes reject a changed physical input mapping',t=>{
  const {second,alias}=fixture(t), {system}=tracked();
  const probe=path.join(alias,'missing.ts');
  assert.equal(system.fileExists(probe),false);
  fs.unlinkSync(alias);fs.symlinkSync(second,alias,'dir');
  assert.throws(()=>system.fileExists(probe),/changed/);
});
