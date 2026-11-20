'use strict';
const fs=require('node:fs');
const path=require('node:path');
const {isDeepStrictEqual}=require('node:util');
const {prepare}=require('./config.js');
const {diagnostics,toolchain,RestartRequired}=require('./compiler.js');
const {Observations}=require('./observations.js');
const {extract}=require('./extract.js');
const VERSION=1, MAX_REQUEST=65536;
function writeOutput(output,result) {
  const temporary=`${output}.tmp`;
  fs.writeFileSync(temporary,JSON.stringify(result));
  fs.renameSync(temporary,output);
}
function exactMembership(metadata) {
  const {watch_roots,output_roots,...membership}=metadata;
  return membership;
}
function verifyCompilerInputs(context) {
  context.observations.verify(key=>{
    // The compiler also enumerates directories to set up wildcard watches.
    // Re-evaluate its selected inputs when such a listing changes.
    const current=prepare(context.config,context);
    current.observations.verify();
    if (!isDeepStrictEqual(exactMembership(current.metadata),exactMembership(context.metadata))) {
      throw new Error(`TypeScript input changed after preparation: ${key}`);
    }
    context.metadata.watch_roots=current.metadata.watch_roots;
  });
}
function analyzed(context) {
  verifyCompilerInputs(context);
  diagnostics(context);
  const result=extract(context);
  verifyCompilerInputs(context);
  return result;
}
function probePath(key) {
  try {
    const probe=JSON.parse(key);
    if (!Array.isArray(probe)) return undefined;
    const file=probe[1];
    if (typeof file!=='string') return undefined;
    return path.isAbsolute(file)?file:undefined;
  } catch { return undefined; }
}
function changedProbe(error) {
  const prefixes=['TypeScript input changed after preparation: ','TypeScript input changed during preparation: '];
  const prefix=prefixes.find(value=>error.message.startsWith(value));
  if (!prefix) return undefined;
  const key=error.message.slice(prefix.length);
  for (const label of ['compiler:','compiler-package:','compiler-lookup:','compatibility:']) {
    if (key.startsWith(label)) return {code:'inputs_changed',path:key.slice(label.length)};
  }
  const file=probePath(key);
  if (file) return {code:'inputs_changed',path:file};
  return {code:'inputs_changed'};
}
function failure(error) {
  const restart=error instanceof RestartRequired;
  return {status:restart?'restart':'error',error:{
    code:restart?'restart_required':'analysis_failed',path:error.path,...changedProbe(error),message:error.message,
  }};
}
function validateSequence(request,previous) {
  if (!Number.isSafeInteger(request.sequence)) throw new Error('Invalid TypeScript worker request sequence');
  if (request.sequence!==previous+1) throw new Error('Invalid TypeScript worker request sequence');
}
class Session {
  constructor(config,output) {
    this.config=config; this.output=output; this.sequence=0; this.context=undefined;
  }
  respond(operation) {
    if (this.context) this.context.affected=null;
    let response;
    try { response=operation(); }
    catch (error) { response={...failure(error),affected:this.context?.affected??null,metadata:this.context?.metadata}; }
    writeOutput(this.output,{version:VERSION,sequence:this.sequence,...response});
  }
  update() {
    // Commit the new native builder before diagnostics: a semantic error is a
    // valid epoch and can be repaired by a subsequent update in this process.
    this.context=prepare(this.config,this.context);
    const affected=diagnostics(this.context);
    verifyCompilerInputs(this.context);
    return {status:'success',metadata:this.context.metadata,affected};
  }
  analyze() {
    if (!this.context) throw new Error('TypeScript worker requires a successful preparation epoch');
    // A loaded module must never outlive its compiler/policy identity. Check
    // before ordinary input verification so the client receives restart status.
    toolchain(this.config,new Observations(),this.context.toolchain);
    const graph=analyzed(this.context);
    return {status:'success',metadata:this.context.metadata,graph,affected:this.context.affected};
  }
  verify() {
    if (!this.context) throw new Error('TypeScript worker requires a successful preparation epoch');
    toolchain(this.config,new Observations(),this.context.toolchain);
    verifyCompilerInputs(this.context);
    return {status:'success',metadata:this.context.metadata};
  }
  request(bytes) {
    const request=JSON.parse(bytes.toString('utf8'));
    if (request.version!==VERSION) throw new Error('Invalid TypeScript worker request version');
    validateSequence(request,this.sequence);
    if (!['update','analyze','verify'].includes(request.command)) throw new Error('Invalid TypeScript worker request command');
    this.sequence=request.sequence;
    this.respond(()=>this[request.command]());
  }
}
class CommandInput {
  constructor(session) { this.session=session; this.pending=Buffer.alloc(0); }
  push(chunk) {
    this.pending=Buffer.concat([this.pending,chunk]);
    let newline;
    while ((newline=this.pending.indexOf(10))!==-1) {
      if (newline>MAX_REQUEST) throw new Error('TypeScript worker request exceeds 64 KiB');
      this.session.request(this.pending.subarray(0,newline));
      this.pending=this.pending.subarray(newline+1);
    }
    if (this.pending.length>MAX_REQUEST) throw new Error('TypeScript worker request exceeds 64 KiB');
  }
  end() {
    if (this.pending.length) throw new Error('Incomplete TypeScript worker request');
  }
}
function worker(config,output) {
  const session=new Session(config,output);
  session.respond(()=>session.update());
  const commands=new CommandInput(session);
  const guarded=operation=>{
    try { operation(); }
    catch (error) {
      session.respond(()=>{throw error;});
      console.error(error.message);
      process.exitCode=1;
      process.stdin.destroy();
    }
  };
  process.stdin.on('data',chunk=>guarded(()=>commands.push(chunk)));
  process.stdin.on('end',()=>guarded(()=>commands.end()));
}
module.exports={worker,analyzed,writeOutput};
