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
function installedDependency(file) {
  return file.split(path.sep).some(part=>part==='node_modules'||part==='.node_modules');
}
function mappedDependencyPaths(file, mappings, observations) {
  const paths=new Set();
  for(const {local,physical} of mappings) {
    if(!within(file,physical)) continue;
    paths.add(file);
    const candidate=path.join(local,path.relative(physical,file));
    const resolved=observations.observe(JSON.stringify(['dependency-alias',candidate]),()=>canonical(candidate));
    if(resolved===file) paths.add(candidate);
  }
  return paths;
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
  const result=new Set(), dependencies=new Set();
  for(const input of inputs) {
    const lexical=projectDependencyPath(input,projects);
    result.add(lexical);
    const normalized=canonical(input);
    if(installedDependency(lexical)) {
      dependencies.add(lexical);
      dependencies.add(normalized);
    }
    for(const dependency of mappedDependencyPaths(normalized,mappings,observations)) {
      result.add(dependency);
      dependencies.add(dependency);
    }
  }
  return {inputs:[...result].sort(),dependencies:[...dependencies].filter(file=>result.has(file)).sort()};
}
module.exports={dependencyInputs};
