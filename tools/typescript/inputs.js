'use strict';
const fs = require('node:fs');
const path = require('node:path');
const {projectInputs,packageVersion}=require('./dependencies.js');
const {canonical} = require('./paths.js');
const OMITTED = new Set(['.git','node_modules','.next','dist','build','coverage','.backlog','.serena','target']);
function includeRoot(config, pattern) {
  const prefix = pattern.split(/[?*]/,1)[0];
  const candidate = path.resolve(path.dirname(config),prefix);
  if (prefix.endsWith('/')) return candidate;
  return fs.existsSync(candidate) && fs.statSync(candidate).isDirectory() ? candidate : path.dirname(candidate);
}
function excludedOutput(directory, outputs) {
  return outputs.some(output=>directory === output || directory.startsWith(`${output}${path.sep}`));
}
function descendants(directory, outputs, result) {
  if (!fs.existsSync(directory)) return;
  if (!fs.statSync(directory).isDirectory()) return;
  directory = canonical(directory);
  if (result.has(directory) || excludedOutput(directory,outputs)) return;
  result.add(directory);
  for (const entry of fs.readdirSync(directory,{withFileTypes:true})) {
    if (!entry.isDirectory() || OMITTED.has(entry.name)) continue;
    descendants(path.join(directory,entry.name),outputs,result);
  }
}
function watchExplicitRoot(root, outputs, result) {
  root = canonical(root);
  if (!fs.existsSync(root)) {
    result.add(root);
    return;
  }
  // Explicit sources may live below an otherwise excluded compiler output.
  const exclusions = outputs.filter(output=>!excludedOutput(root,[output]));
  descendants(root,exclusions,result);
}
function watchRoots(projects, owners) {
  const roots = new Set([...owners.keys()].map(source=>path.dirname(source)));
  const outputs = [];
  for (const {config,parsed} of projects.values()) {
    const includes = (parsed.raw.include || []).map(pattern=>includeRoot(config,pattern));
    const wildcards = Object.keys(parsed.wildcardDirectories||{});
    [path.dirname(config),...includes,...wildcards].forEach(root=>roots.add(root));
    outputs.push(parsed.options.outDir,parsed.options.declarationDir);
  }
  const result = new Set();
  const canonicalOutputs = outputs.filter(Boolean).map(canonical);
  for (const root of roots) watchExplicitRoot(root,canonicalOutputs,result);
  return [...result].sort();
}
function outputRoots(projects) {
  return [...projects.values()].flatMap(({config,parsed})=>[
    parsed.options.outDir, parsed.options.declarationDir,
    ...['.next','dist','coverage'].map(name=>path.join(path.dirname(config),name)),
  ]).filter(Boolean).map(canonical);
}
function metadata(context, inputs) {
  const {ts,projects,owners,programs} = context;
  const versions = {typescript:ts.version,node:process.versions.node,adapter:'1',react_rules:'1',next_rules:'1'};
  for (const {config} of projects.values()) {
    projectInputs(config,inputs);
    for (const name of ['typescript','react','next']) packageVersion(config,name,inputs,versions);
  }
  for (const {program} of programs) {
    for (const source of program.getSourceFiles()) inputs.add(canonical(source.fileName));
  }
  return {configs:[...projects.keys()].sort(),sources:[...owners.keys()].sort(),owners:Object.fromEntries(owners),
    inputs:[...inputs].sort(),watch_roots:watchRoots(projects,owners),output_roots:outputRoots(projects),versions};
}
module.exports = {metadata};
