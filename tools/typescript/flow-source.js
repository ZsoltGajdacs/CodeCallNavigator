'use strict';
const {isCallable,isCallSite,isAccess}=require('./syntax.js');
const {accessorTargets}=require('./accessors.js');
const {FlowLimits}=require('./flow-limits.js');
/** Translate compiler UTF-16 offsets to the original snapshot's UTF-8 bytes once per file. */
function byteOffsets(text) {
  const offsets=new Uint32Array(text.length+1);
  let bytes=0;
  for(let i=0;i<text.length;) {
    const value=text.codePointAt(i),width=value>0xffff?2:1;
    offsets[i]=bytes;
    if(width===2) offsets[i+1]=bytes;
    bytes+=Buffer.byteLength(String.fromCodePoint(value),'utf8');
    i+=width;offsets[i]=bytes;
  }
  return offsets;
}
const sourceOffsetsCache=new WeakMap();
function sourceOffsets(file) {
  if(!sourceOffsetsCache.has(file)) sourceOffsetsCache.set(file,byteOffsets(file.text));
  return sourceOffsetsCache.get(file);
}
class FlowSource {
  constructor(bindings,{file,checker,owner}) {
    this.bindings=bindings;this.ts=bindings.ts;this.file=file;this.checker=checker;this.owner=owner;
    this.offsets=sourceOffsets(file);this.limits=new FlowLimits();
  }
  span(node) {return {start:this.offsets[node.getStart(this.file)],end:this.offsets[node.end]};}
  text(node) {return node.getText(this.file);}
  target(index,indices,propagated) {
    const node=this.bindings.graph.nodes[index];
    const possible=propagated||indices.length>1||node.body_unavailable;
    return {symbol:node.id,certainty:possible?'possible':'direct',provenance:'typescript.compiler'};
  }
  label(expression) {
    if(!expression) return 'call';
    return expression.name?expression.name.getText(this.file):expression.getText(this.file);
  }
  call(node) {
    const expression=node.expression||node.tag;
    const targets=this.bindings.callTargets(this.checker,node)
      .filter(target=>!this.bindings.graph.excluded.has(target.index))
      .map(({index,certainty,provenance})=>({symbol:this.bindings.graph.nodes[index].id,certainty,provenance}));
    return {span:this.span(node),label:this.label(expression),targets,
      diagnostic:targets.length?null:'No statically bound project target'};
  }
  accessor(node) {
    const indices=accessorTargets(this.bindings,this.checker,node).filter(i=>!this.bindings.graph.excluded.has(i));
    if(!indices.length) return [];
    return [{span:this.span(node),label:this.text(node),targets:indices.map(i=>this.target(i,[i],false)),diagnostic:null}];
  }
  ownCalls(node) {
    if(isCallSite(this.ts,node)) return [this.call(node)];
    return isAccess(this.ts,node)?this.accessor(node):[];
  }
  calls(node) {
    const calls=[];
    const visit=current=>{
      if(isCallable(this.ts,current)) return;
      this.ts.forEachChild(current,visit);
      calls.push(...this.ownCalls(current));
    };
    visit(node);return calls;
  }
}
module.exports={FlowSource,byteOffsets,sourceOffsets};
