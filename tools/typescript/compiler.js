'use strict';
const path=require('node:path');
const fs=require('node:fs');
const {createRequire}=require('node:module');
const {createHash}=require('node:crypto');
const {canonical}=require('./paths.js');
const {supportsVersion,supportedVersions,loadCompatibility}=require('./compatibility.js');
function failDiagnostics(ts, diagnostics) {
  const errors = diagnostics.filter(d => d.category === ts.DiagnosticCategory.Error);
  if (!errors.length) return;
  throw new Error(ts.formatDiagnostics(errors, {getCanonicalFileName:f=>f,getCurrentDirectory:()=>process.cwd(),getNewLine:()=> '\n'}));
}
class RestartRequired extends Error {
  constructor(message,file) { super(message); this.path=file; }
}
function checksum(value) { return createHash('sha256').update(value).digest('hex'); }
function toolchain(config, observations, previous) {
  try { return loadToolchain(config,observations,previous); }
  catch (error) {
    if (previous && !(error instanceof RestartRequired)) throw new RestartRequired(`Cannot revalidate TypeScript toolchain; restart the worker: ${error.message}`,error.path||previous.manifest||previous.entry);
    throw error;
  }
}
function compilerModule(localRequire,entry,identity,supportedMinors) {
  const ts=localRequire(entry);
  if (ts.version!==identity.version) throw new RestartRequired('Loaded TypeScript version differs from its installed package; restart the worker',identity.manifest);
  if (!supportsVersion(ts.version,supportedMinors)) throw new Error(`Unsupported TypeScript ${ts.version}; expected project-installed ${supportedVersions(supportedMinors)}`);
  return ts;
}
function packageBinding(candidate) {
  try { return fs.realpathSync(candidate); }
  catch (error) {
    if (['ENOENT','ENOTDIR'].includes(error.code)) return null;
    throw error;
  }
}
function compilerLookups(localRequire,observations,selectedRoot) {
  const lookups=[];
  for (const directory of localRequire.resolve.paths('typescript')) {
    const candidate=path.join(directory,'typescript');
    const binding=observations.observe(`compiler-lookup:${candidate}`,()=>packageBinding(candidate));
    lookups.push({path:path.join(candidate,'package.json'),binding});
    // Later native search roots cannot affect an already selected package.
    if (binding===selectedRoot) break;
  }
  return lookups;
}
function compatibility(observations) {
  try { return loadCompatibility(observations); }
  catch (error) { error.path=path.join(__dirname,'compatibility.json'); throw error; }
}
function toolchainComponents(identity,config) {
  return [
    {key:'node',file:config,value:identity.node},
    {key:'policy',file:identity.policy,value:[identity.policy,identity.policy_checksum]},
    ...identity.lookups.map(lookup=>({key:`lookup:${lookup.path}`,file:lookup.path,value:lookup.binding})),
    {key:'manifest',file:identity.manifest,value:[identity.manifest,identity.package_checksum,identity.version]},
    {key:'entry',file:identity.entry,value:[identity.entry,identity.checksum]},
  ];
}
function changedToolchain(previous,current,config) {
  const before=new Map(toolchainComponents(previous,config).map(component=>[component.key,component]));
  for (const component of toolchainComponents(current,config)) {
    if (JSON.stringify(before.get(component.key)?.value)!==JSON.stringify(component.value)) return component.file;
    before.delete(component.key);
  }
  return before.values().next().value?.file;
}
function loadToolchain(config, observations, previous) {
  if (Number(process.versions.node.split('.')[0]) < 24) throw new Error('TypeScript analysis requires Node 24+');
  const {policyFile,supportedMinors}=compatibility(observations);
  const expected=supportedVersions(supportedMinors);
  const localRequire = createRequire(path.join(path.dirname(config), '__codecallnavigator__.js'));
  let entry;
  try { entry = localRequire.resolve('typescript'); }
  catch { throw new Error(`Install project TypeScript ${expected} dependencies before analyzing ${config}`); }
  const contents=observations.observe(`compiler:${entry}`,()=>fs.readFileSync(entry,'utf8'));
  const policy=observations.observe(`compatibility:${policyFile}`,()=>fs.readFileSync(policyFile,'utf8'));
  const packageFile=localRequire.resolve('typescript/package.json');
  const packageContents=observations.observe(`compiler-package:${packageFile}`,()=>fs.readFileSync(packageFile,'utf8'));
  const identity={entry:canonical(entry),checksum:checksum(contents),version:JSON.parse(packageContents).version,manifest:canonical(packageFile),
    package_checksum:checksum(packageContents),lookups:compilerLookups(localRequire,observations,canonical(path.dirname(packageFile))),
    policy:canonical(policyFile),policy_checksum:checksum(policy),node:process.versions.node};
  const changed=previous && changedToolchain(previous,identity,config);
  if (changed) throw new RestartRequired('TypeScript toolchain entry, checksum or compatibility policy changed; restart the worker',changed);
  const ts=compilerModule(localRequire,entry,identity,supportedMinors);
  const inputPaths=[entry,canonical(entry),policyFile,canonical(policyFile),packageFile,canonical(packageFile),
    ...identity.lookups.map(lookup=>lookup.path)];
  return {ts,localRequire,identity,inputPaths};
}
function trackingSystem(ts, inputs, observations) {
  const track = file => {
    if (!path.isAbsolute(file)) return;
    // Preserve the original compiler probes below. Observe this mapping too:
    // a symlink may change destination while both fileExists results stay false.
    const normalized=observations.observe(JSON.stringify(['input-path',file]),()=>canonical(file));
    inputs.add(normalized);
    // Preserve compiler-required aliases as inputs too; source ownership keeps
    // canonical paths, while freshness records lexical symlink identity.
    const lexical=path.resolve(file);
    if (lexical!==normalized) inputs.add(lexical);
  };
  const system={...ts.sys};
  for (const name of ['readFile','fileExists','directoryExists','readDirectory','getDirectories','realpath']) {
    if (!ts.sys[name]) continue;
    system[name]=(...args)=>{
      if (name==='readFile' || name==='fileExists') track(args[0]);
      const key=JSON.stringify([name,...args]);
      return observations.observe(key,()=>ts.sys[name](...args),name==='getDirectories');
    };
  }
  system.writeFile=()=>{ throw new Error('TypeScript analysis must never emit files'); };
  system.watchFile=()=>({close(){}});
  system.watchDirectory=()=>({close(){}});
  return system;
}
function createPrograms(ts, projects, system, previous=[]) {
  const retained=new Map(previous.map(item=>[item.config,item.builder]));
  const programs = [];
  for (const project of projects.values()) {
    let builder;
    const diagnostics = [];
    const create=(roots,options,host,_old,errors,references)=>{
      // Keep each epoch's compiler reads complete. The root-name builder
      // overload also reuses the old Program's resolution state.
      const program=ts.createProgram({
        rootNames:roots,options,host,configFileParsingDiagnostics:errors,projectReferences:references,
      });
      return ts.createSemanticDiagnosticsBuilderProgram(program,host,retained.get(project.config),errors);
    };
    const host = ts.createWatchCompilerHost(project.config, {noEmit:true}, system, create, d=>diagnostics.push(d), ()=>{});
    host.useSourceOfProjectReferenceRedirect = () => true;
    host.afterProgramCreate = current => { builder = current; };
    const watch = ts.createWatchProgram(host);
    watch.close();
    failDiagnostics(ts,diagnostics);
    if (!builder) throw new Error(`Compiler program unavailable: ${project.config}`);
    programs.push({config:project.config,program:builder.getProgram(),builder});
  }
  return programs;
}
function affectedDiagnostics(builder) {
  let affected=0;
  // Count actual native affected diagnostic work items, including a Program
  // item for a bundle. This is not a guess from SourceFile object identity.
  while (builder.getSemanticDiagnosticsOfNextAffectedFile()) affected++;
  return affected;
}
function programDiagnostics(ts,builder) {
  const result=[builder.getConfigFileParsingDiagnostics(),builder.getOptionsDiagnostics(),
    builder.getSyntacticDiagnostics(),builder.getGlobalDiagnostics(),builder.getSemanticDiagnostics()];
  // These runtime exports are pinned by the supported TypeScript minor policy.
  // Match getPreEmitDiagnostics even with noEmit and declaration/composite refs.
  if (ts.getEmitDeclarations(builder.getCompilerOptions())) result.push(builder.getDeclarationDiagnostics());
  return result.flat();
}
function diagnostics(context) {
  let affected=0, result=[];
  for (const {builder} of context.programs) {
    affected+=affectedDiagnostics(builder);
    result=result.concat(programDiagnostics(context.ts,builder));
  }
  context.affected=affected;
  failDiagnostics(context.ts,context.ts.sortAndDeduplicateDiagnostics(result));
  return affected;
}
module.exports={failDiagnostics,toolchain,trackingSystem,createPrograms,diagnostics,RestartRequired};
