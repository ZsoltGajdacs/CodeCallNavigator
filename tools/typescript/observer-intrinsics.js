'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {compilerGlobal,globalMemberUse,outerExpression}=require('./intrinsic-globals.js');
const SET_PROOF='codegraph.observer:builtin-set-escape';
function iterator(ts,declaration) {
  if(!declaration.name||!ts.isComputedPropertyName(declaration.name)) return false;
  const expression=declaration.name.expression;
  return ts.isPropertyAccessExpression(expression)&&expression.name.text==='iterator';
}
function members(bindings,type) {
  const relevant=symbol=>symbol.name==='add'||symbol.declarations?.some(declaration=>iterator(bindings.ts,declaration));
  return type.getProperties().filter(relevant).flatMap(symbol=>(symbol.declarations||[]).map(declarationKey));
}
function storageMembers(bindings,checker,declaration) {
  const constructor=syntax.keys(bindings,checker,declaration.initializer.expression);
  return [SET_PROOF,...constructor,...members(bindings,checker.getTypeAtLocation(declaration.name))];
}
function elementWrites(bindings,node,checker) {
  const type=checker.getTypeAtLocation(syntax.unwrap(bindings.ts,node.expression));
  const symbol=type.getSymbol();
  if(symbol?.name!=='Set'||!syntax.librarySymbol(bindings,symbol)) return [];
  return members(bindings,type);
}
function constructorUseEscapes(ts,node) {
  const expression=outerExpression(ts,node),parent=expression.parent;
  return !ts.isNewExpression(parent)||parent.expression!==expression;
}
function setEscape(bindings,checker,node) {
  if(syntax.typePosition(bindings.ts,node)) return false;
  const use=globalMemberUse(bindings,checker,node,'Set');
  if(use!==undefined) return use===null||constructorUseEscapes(bindings.ts,use);
  if(node.text!=='Set'||!compilerGlobal(bindings,checker,node)) return false;
  return constructorUseEscapes(bindings.ts,node);
}
function validateStorage(bindings,channels,writes) {
  for(const channel of channels.values()) {
    if(!channel.valid||!channel.intrinsics.some(key=>writes.has(key))) continue;
    channel.valid=false;
    bindings.graph.diagnostic('unresolved observer builtin storage mutation');
  }
}
module.exports={storageMembers,elementWrites,validateStorage,setEscape,SET_PROOF};
