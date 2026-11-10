'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {written}=require('./observer-values.js');
function collectFacts(bindings) {
  const writes=new Set(),calls=[];
  syntax.walkOwned(bindings,(node,{checker})=>{
    if(bindings.ts.isCallExpression(node)) calls.push({node,checker});
    if(!bindings.ts.isIdentifier(node)||syntax.typePosition(bindings.ts,node)) return;
    if(written(bindings.ts,node)) for(const key of syntax.keys(bindings,checker,node)) writes.add(key);
  });
  return {writes,calls};
}
function within(node,owner) {
  for(let current=node;current;current=current.parent) if(current===owner) return true;
  return false;
}
function capturesFactory(bindings,checker,node,factory) {
  if(!factory||!within(node,factory)) return false;
  let captured=false;
  const visit=current=>{
    if(capturesReference(bindings,checker,current,{factory,returned:node})) captured=true;
    bindings.ts.forEachChild(current,visit);
  };
  visit(node);return captured;
}
function capturesReference(bindings,checker,node,{factory,returned}) {
  if(!bindings.ts.isIdentifier(node)||syntax.typePosition(bindings.ts,node)) return false;
  const declarations=bindings.symbol(checker,node)?.declarations||[];
  return declarations.some(declaration=>within(declaration,factory)&&!within(declaration,returned));
}
function unchanged(bindings,writes,declaration) {
  return !writes.has(declarationKey(declaration))&&!syntax.decoratedOwner(bindings.ts,declaration);
}
module.exports={collectFacts,capturesFactory,unchanged};
