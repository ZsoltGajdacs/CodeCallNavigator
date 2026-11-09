'use strict';
const {failDiagnostics,toolchain,trackingSystem,createPrograms}=require('./compiler.js');
const {canonical,isProjectSource} = require('./paths.js');
const {metadata} = require('./inputs.js');
const {Observations}=require('./observations.js');
function parseProject(ts,file,system) {
  const parsed = ts.getParsedCommandLineOfConfigFile(file, {}, {...system,onUnRecoverableConfigFileDiagnostic:d=>failDiagnostics(ts,[d])});
  if (!parsed) throw new Error(`Cannot parse TypeScript configuration ${file}`);
  failDiagnostics(ts, parsed.errors);
  if (parsed.options.allowJs) throw new Error(`JavaScript/allowJs projects are outside this TypeScript release: ${file}`);
  return parsed;
}
function configuredProjects(ts, config, system) {
  const projects = new Map();
  function visit(file, active = new Set()) {
    file = canonical(file);
    if (active.has(file)) throw new Error(`Circular TypeScript project reference: ${file}`);
    if (projects.has(file)) return;
    const parsed = parseProject(ts,file,system);
    projects.set(file, {config:file,parsed});
    const chain = new Set(active).add(file);
    for (const reference of parsed.projectReferences || []) visit(ts.resolveProjectReferencePath(reference), chain);
  }
  visit(config);
  return projects;
}
function ownedSources(projects) {
  const owners = new Map();
  for (const project of projects.values()) {
    for (const file of project.parsed.fileNames) {
      const source = canonical(file);
      if (!isProjectSource(source)) continue;
      const previous = owners.get(source);
      if (previous && previous !== project.config) throw new Error(`Conflicting TypeScript ownership: ${source} belongs to ${previous} and ${project.config}`);
      owners.set(source,project.config);
    }
  }
  return owners;
}
function prepare(config) {
  config = canonical(config);
  const observations=new Observations();
  const {ts,entry,localRequire} = toolchain(config,observations);
  const inputs = new Set([entry]);
  const system = trackingSystem(ts,inputs,observations);
  const projects = configuredProjects(ts,config,system);
  const owners = ownedSources(projects);
  const programs = createPrograms(ts,projects,system);
  const context = {ts,config,projects,owners,programs,localRequire,observations};
  context.metadata = metadata(context,inputs);
  return context;
}
module.exports={prepare,canonical,failDiagnostics};
