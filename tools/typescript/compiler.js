'use strict';
const path=require('node:path');
const fs=require('node:fs');
const {createRequire}=require('node:module');
const {canonical}=require('./paths.js');
function failDiagnostics(ts, diagnostics) {
  const errors = diagnostics.filter(d => d.category === ts.DiagnosticCategory.Error);
  if (!errors.length) return;
  throw new Error(ts.formatDiagnostics(errors, {getCanonicalFileName:f=>f,getCurrentDirectory:()=>process.cwd(),getNewLine:()=> '\n'}));
}
function toolchain(config, observations) {
  if (Number(process.versions.node.split('.')[0]) < 24) throw new Error('TypeScript analysis requires Node 24+');
  const localRequire = createRequire(path.join(path.dirname(config), '__codecallnavigator__.js'));
  let entry;
  try { entry = localRequire.resolve('typescript'); }
  catch { throw new Error(`Install project TypeScript 6.0.x dependencies before analyzing ${config}`); }
  observations.observe(`compiler:${entry}`,()=>fs.readFileSync(entry,'utf8'));
  const ts = localRequire(entry);
  if (!/^6\.0\./.test(ts.version)) throw new Error(`Unsupported TypeScript ${ts.version}; expected project-installed 6.0.x`);
  return { ts, entry: canonical(entry), localRequire };
}
function trackingSystem(ts, inputs, observations) {
  const track = file => { if (path.isAbsolute(file)) inputs.add(canonical(file)); };
  const system={...ts.sys};
  for (const name of ['readFile','fileExists','directoryExists','readDirectory','getDirectories','realpath']) {
    if (!ts.sys[name]) continue;
    system[name]=(...args)=>{
      if (name==='readFile' || name==='fileExists') track(args[0]);
      const key=JSON.stringify([name,...args]);
      return observations.observe(key,()=>ts.sys[name](...args));
    };
  }
  system.writeFile=()=>{ throw new Error('TypeScript analysis must never emit files'); };
  system.watchFile=()=>({close(){}});
  system.watchDirectory=()=>({close(){}});
  return system;
}
function createPrograms(ts, projects, system) {
  const programs = [];
  for (const project of projects.values()) {
    let program;
    const diagnostics = [];
    const host = ts.createWatchCompilerHost(project.config, {noEmit:true}, system, ts.createSemanticDiagnosticsBuilderProgram, d=>diagnostics.push(d), ()=>{});
    host.useSourceOfProjectReferenceRedirect = () => true;
    host.afterProgramCreate = builder => { program = builder.getProgram(); };
    const watch = ts.createWatchProgram(host);
    watch.close();
    failDiagnostics(ts,diagnostics);
    if (!program) throw new Error(`Compiler program unavailable: ${project.config}`);
    programs.push({config:project.config,program});
  }
  return programs;
}
module.exports={failDiagnostics,toolchain,trackingSystem,createPrograms};
