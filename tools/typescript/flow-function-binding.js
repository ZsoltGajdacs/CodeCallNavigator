'use strict';
const {unchanged}=require('./callable-facts.js');

function typeOnly(declaration) {
  for(let node=declaration;node;node=node.parent) if(node.isTypeOnly) return true;
  return false;
}
function simpleCall(ts,node) {
  return ts.isCallExpression(node)&&!ts.isOptionalChain(node)&&ts.isIdentifier(node.expression);
}
function runtimeDeclarations(source,node) {
  const {checker,bindings}=source,local=checker.getSymbolAtLocation(node);
  if(!local) return [];
  const declarations=local.getDeclarations()||[];
  if(declarations.some(typeOnly)) return [];
  return bindings.unalias(checker,local).getDeclarations()||[];
}
function uniqueBody(ts,declarations) {
  const bodies=declarations.filter(declaration=>ts.isFunctionDeclaration(declaration)&&declaration.body);
  return bodies.length===1?bodies[0]:undefined;
}
function resolvedSignature(source,node,declarations) {
  const signature=source.checker.getResolvedSignature(node)?.declaration;
  return declarations.includes(signature)&&!signature.getSourceFile().isDeclarationFile;
}
function ownedBody(source,body) {
  if(body.getSourceFile().isDeclarationFile) return false;
  if(source.bindings.exportIndex(body)===undefined) return false;
  return unchangedBody(source,body);
}
function unchangedBody(source,body) {
  const writes=source.bindings.callables?.writes;
  return !!writes&&unchanged(source.bindings,writes,body);
}
/** Compiler declaration ownership, independent of target count, certainty or graph pruning. */
function implementation(source,node) {
  if(!simpleCall(source.ts,node)) return undefined;
  const declarations=runtimeDeclarations(source,node.expression),body=uniqueBody(source.ts,declarations);
  if(!body||!resolvedSignature(source,node,declarations)) return undefined;
  return ownedBody(source,body)?body:undefined;
}
module.exports={implementation};
