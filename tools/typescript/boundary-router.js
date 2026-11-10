'use strict';
const {declarationKey}=require('./paths.js');
function constVariable(ts,declaration) {
  if(!ts.isVariableDeclaration(declaration)) return false;
  if(!ts.isVariableDeclarationList(declaration.parent)) return false;
  return Boolean(declaration.parent.flags&ts.NodeFlags.Const);
}
function routerDeclaration(bindings,checker,receiver,writes) {
  if(!bindings.ts.isIdentifier(receiver)) return undefined;
  const declarations=bindings.symbol(checker,receiver)?.declarations||[];
  if(declarations.length!==1) return undefined;
  const declaration=declarations[0],{ts}=bindings;
  if(!constVariable(ts,declaration)) return undefined;
  return writes.has(declarationKey(declaration))?undefined:declaration;
}
function ordinaryReceiverUse(ts,node) {
  const property=node.parent;
  if(!ts.isPropertyAccessExpression(property)||property.expression!==node) return false;
  const call=property.parent;
  return ts.isCallExpression(call)&&call.expression===property;
}
function referencesDeclaration(bindings,checker,node,declaration) {
  if(!bindings.ts.isIdentifier(node)||node===declaration.name) return false;
  return bindings.symbol(checker,node)?.declarations?.includes(declaration);
}
function escaped(bindings,declaration) {
  let unsafe=false;
  for(const {file,checker} of bindings.files) {
    const visit=node=>{
      if(referencesDeclaration(bindings,checker,node,declaration)&&!ordinaryReceiverUse(bindings.ts,node)) unsafe=true;
      bindings.ts.forEachChild(node,visit);
    };
    visit(file);
  }
  return unsafe;
}
module.exports={routerDeclaration,escaped};
