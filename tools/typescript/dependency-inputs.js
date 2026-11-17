'use strict';
const path=require('node:path');
const {canonical}=require('./paths.js');

function within(file,root) {
  return file===root || file.startsWith(`${root}${path.sep}`);
}
function projectDependencyPath(file, projects) {
  const parts=file.split(path.sep);
  for(let index=0;index<parts.length;index++) {
    if(parts[index]!=='node_modules') continue;
    const prefix=canonical(parts.slice(0,index).join(path.sep) || path.parse(file).root);
    if(projects.has(prefix)) return path.join(prefix,...parts.slice(index));
  }
  return file;
}
// Emit a stable set regardless of whether the compiler reached a dependency via
// its physical installation or the project's node_modules link. Original probes
// remain in Observations: normalization must not weaken in-run verification.
function dependencyInputs(inputs, projectDirectories, observations) {
  const projects=new Set(projectDirectories.map(canonical));
  const mappings=[...projects].sort().map(project=>{
    const local=path.join(project,'node_modules');
    const physical=observations.observe(JSON.stringify(['dependency-root',local]),()=>canonical(local));
    return {local,physical};
  });
  const result=new Set();
  for(const input of inputs) {
    result.add(projectDependencyPath(input,projects));
    const normalized=canonical(input);
    for(const {local,physical} of mappings) {
      if(within(normalized,physical)) {
        result.add(normalized);
        const candidate=path.join(local,path.relative(physical,normalized));
        const resolved=observations.observe(JSON.stringify(['dependency-alias',candidate]),()=>canonical(candidate));
        if(resolved===normalized) result.add(candidate);
      }
    }
  }
  return [...result].sort();
}
module.exports={dependencyInputs};
