'use strict';
const {isCallable}=require('./syntax.js');
const {MAX_FLOW_NODES}=require('./flow-limits.js');
/** Whole-body compiler flags, independent of Promise names and caller activation. */
function execution(ts,declaration) {
  if(!implemented(ts,declaration)) return undefined;
  if(ts.getCombinedModifierFlags(declaration)&ts.ModifierFlags.Async) return undefined;
  const suspension=ownedSuspension(ts,declaration);
  if(suspension.has('unknown')||suspension.has('await')) return undefined;
  if(declaration.asteriskToken) return declaration.parameters.some(parameter=>parameter.initializer)?undefined:'deferred';
  return suspension.has('yield')?undefined:'synchronous';
}
function implemented(ts,declaration) {
  return !!declaration.body&&!declaration.getSourceFile().isDeclarationFile
    &&(ts.isFunctionDeclaration(declaration)||ts.isMethodDeclaration(declaration));
}
function awaits(ts,node) {return ts.isAwaitExpression(node)||(ts.isForOfStatement(node)&&!!node.awaitModifier);}
function suspensionKind(ts,node) {
  if(awaits(ts,node)) return 'await';
  return ts.isYieldExpression(node)?'yield':undefined;
}
function ownedSuspension(ts,declaration) {
  const result=new Set(),pending=[declaration];
  let visited=0;
  while(pending.length) {
    if(++visited>MAX_FLOW_NODES) {result.add('unknown');break;}
    const node=pending.pop();
    if(node!==declaration&&isCallable(ts,node)) continue;
    const kind=suspensionKind(ts,node);
    if(kind) result.add(kind);
    ts.forEachChild(node,child=>{pending.push(child);});
  }
  return result;
}
module.exports={execution};
