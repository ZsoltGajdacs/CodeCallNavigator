'use strict';
const fs=require('node:fs');
const path=require('node:path');
function supportsVersion(version, supportedMinors) {
  if (typeof version !== 'string') return false;
  const match=/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.exec(version);
  return Boolean(match && match[0]===version && supportedMinors.includes(`${match[1]}.${match[2]}`));
}
function supportedVersions(supportedMinors) {
  return supportedMinors.map(minor=>`${minor}.x`).join(' or ');
}
function loadCompatibility(observations) {
  const policyFile=path.join(__dirname,'compatibility.json');
  const contents=observations.observe(`compatibility:${policyFile}`,()=>fs.readFileSync(policyFile,'utf8'));
  const {supported_minors:supportedMinors}=JSON.parse(contents);
  return {policyFile,supportedMinors};
}
module.exports={supportsVersion,supportedVersions,loadCompatibility};
