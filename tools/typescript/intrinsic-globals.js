'use strict';
const {isWrapper}=require('./syntax.js');
const ROOTS=new Set(['globalThis','window','self','global']);
function standardDeclaration(bindings,declaration) {
  const file=declaration.getSourceFile();
  if(bindings.context.programs.some(({program})=>program.isSourceFileDefaultLibrary(file))) return true;
  return /\/node_modules\/@types\/node\//.test(file.fileName);
}
function globalReference(bindings,checker,node) {
  if(!ROOTS.has(node.text)) return false;
  return compilerGlobal(bindings,checker,node);
}
function propertyNameReference(ts,node) {return ts.isPropertyAccessExpression(node.parent)&&node.parent.name===node;}
function compilerGlobal(bindings,checker,node) {
  if(!bindings.ts.isIdentifier(node)||propertyNameReference(bindings.ts,node)) return false;
  const actual=checker.getSymbolAtLocation(node);
  if(!actual||actual!==checker.resolveName(node.text,undefined,bindings.ts.SymbolFlags.Value,false)) return false;
  if(node.text==='globalThis') return true;
  const declarations=actual.declarations||[];
  return declarations.length>0&&declarations.every(declaration=>standardDeclaration(bindings,declaration));
}
function outerExpression(ts,node) {
  while(isWrapper(ts,node.parent)&&node.parent.expression===node) node=node.parent;
  return node;
}
function receiverProperty(ts,node) {
  const parent=node.parent;
  if(ts.isPropertyAccessExpression(parent)) return parent.expression===node?parent:undefined;
  if(ts.isElementAccessExpression(parent)) return parent.expression===node?parent:undefined;
  return undefined;
}
function propertyName(ts,property) {
  if(ts.isPropertyAccessExpression(property)) return property.name.text;
  return ts.isStringLiteralLike(property.argumentExpression)?property.argumentExpression.text:undefined;
}
function globalReceiver(ts,node) {
  node=outerExpression(ts,node);
  let property=receiverProperty(ts,node);
  while(property&&ROOTS.has(propertyName(ts,property))) {
    node=outerExpression(ts,property);
    property=receiverProperty(ts,node);
  }
  return node;
}
// Undefined is unrelated; null means the global object or an unknown member escaped.
function globalMemberUse(bindings,checker,node,name) {
  if(!globalReference(bindings,checker,node)) return undefined;
  const property=receiverProperty(bindings.ts,globalReceiver(bindings.ts,node));
  if(!property) return null;
  const member=propertyName(bindings.ts,property);
  if(member===undefined) return null;
  return member===name?property:undefined;
}
module.exports={compilerGlobal,globalMemberUse,outerExpression};
