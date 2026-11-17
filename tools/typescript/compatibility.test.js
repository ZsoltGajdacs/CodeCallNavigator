'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {supportsVersion,supportedVersions}=require('./compatibility.js');
const minors=['5.9','6.0'];
test('stable compiler versions accept only supported minor families',()=>{
  for(const version of ['5.9.0','5.9.3','5.9.99','6.0.0','6.0.3']) assert.equal(supportsVersion(version,minors),true,version);
  for(const version of ['5.8.3','6.1.0','7.0.0','5.9','v5.9.3','5.9.3-beta','6.0.0-dev.1','5.9.3+build','5.9.3\n','05.9.3','5.09.3','5.9.03','5.9.3.1','',null,undefined,593]) assert.equal(supportsVersion(version,minors),false,version);
});
test('compiler support labels follow the supplied policy',()=>{
  assert.equal(supportedVersions(minors),'5.9.x or 6.0.x');
  assert.equal(supportedVersions(['6.0']),'6.0.x');
  assert.equal(supportsVersion('5.9.3',['6.0']),false);
});
