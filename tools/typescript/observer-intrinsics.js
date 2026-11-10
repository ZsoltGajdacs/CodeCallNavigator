'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
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
  return [...constructor,...members(bindings,checker.getTypeAtLocation(declaration.name))];
}
function elementWrites(bindings,node,checker) {
  const type=checker.getTypeAtLocation(syntax.unwrap(bindings.ts,node.expression));
  const symbol=type.getSymbol();
  if(symbol?.name!=='Set'||!syntax.librarySymbol(bindings,symbol)) return [];
  return members(bindings,type);
}
function validateStorage(bindings,channels,writes) {
  for(const channel of channels.values()) {
    if(!channel.valid||!channel.intrinsics.some(key=>writes.has(key))) continue;
    channel.valid=false;
    bindings.graph.diagnostic('unresolved observer builtin storage mutation');
  }
}
module.exports={storageMembers,elementWrites,validateStorage};
