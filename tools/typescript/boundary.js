'use strict';
const {sourceSite}=require('./evidence.js');
const {isCallable,isJsxSite}=require('./syntax.js');
const {boundaryWrites}=require('./boundary-writes.js');
const api=require('./boundary-api.js');
const {routes}=require('./boundary-routes.js');
const {pathLiteral}=require('./boundary-path.js');
const {fetchMethod}=require('./boundary-fetch.js');
function callFact(bindings,checker,node,writes) {
  const {ts}=bindings;
  const name=api.nextApi(bindings,checker,node.expression);
  if(['redirect','permanentRedirect'].includes(name)||api.routerMethod(bindings,checker,node.expression,writes)) {
    if(node.arguments.length<1||node.arguments.some(ts.isSpreadElement)) return undefined;
    return {kind:'navigation',path:pathLiteral(ts,node.arguments[0]),method:null,rule:'codecallnavigator.next:navigation',flow:true};
  }
  if(!api.builtinFetch(bindings,checker,node.expression,writes)) return undefined;
  const method=fetchMethod(ts,node);
  return method?{kind:'http',path:pathLiteral(ts,node.arguments[0]),method,rule:'codecallnavigator.typescript:http',flow:true}:undefined;
}
function hrefExpression(ts,initializer) {
  return initializer&&ts.isJsxExpression(initializer)?initializer.expression:initializer;
}
function jsxFact(bindings,checker,node) {
  const {ts}=bindings;
  if(!['default','Link'].includes(api.nextApi(bindings,checker,node.tagName))) return undefined;
  if(node.attributes.properties.some(ts.isJsxSpreadAttribute)) return undefined;
  if(node.attributes.properties.some(property=>property.name?.text==='as')) return undefined;
  const href=node.attributes.properties.filter(property=>property.name?.text==='href');
  if(href.length!==1) return undefined;
  const initializer=href[0].initializer;
  const expression=hrefExpression(ts,initializer);
  return {kind:'navigation',path:pathLiteral(ts,expression),method:null,rule:'codecallnavigator.next:link',flow:false};
}
function nodeOwner(bindings,node,current) {
  if(bindings.ts.isClassLike(node)) return undefined;
  return isCallable(bindings.ts,node)?bindings.exportIndex(node):current;
}
function boundaryFacts(bindings) {
  const result={version:1,routes:[],uses:[]};
  const writes=boundaryWrites(bindings);
  for(const item of bindings.files) {
    result.routes.push(...routes(bindings,item));
    const visit=(node,current)=>{
      current=nodeOwner(bindings,node,current);
      const fact=bindings.ts.isCallExpression(node)?callFact(bindings,item.checker,node,writes):
        isJsxSite(bindings.ts,node)?jsxFact(bindings,item.checker,node):undefined;
      if(fact?.path&&current!==undefined) result.uses.push({...fact,symbol:bindings.graph.nodes[current].id,
        build:item.owner,application:'next-app',site:sourceSite(node)});
      bindings.ts.forEachChild(node,child=>visit(child,current));
    };
    visit(item.file,item.module);
  }
  return result;
}
module.exports={boundaryFacts};
