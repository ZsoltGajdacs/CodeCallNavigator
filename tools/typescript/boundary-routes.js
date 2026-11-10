'use strict';
const {sourceSite}=require('./evidence.js');
const {sourcePath}=require('./paths.js');
const {hasNext}=require('./next.js');
const {appFolders,routePath,configurationRestrictions}=require('./boundary-template.js');
const HTTP=new Set(['GET','POST','PUT','PATCH','DELETE','HEAD','OPTIONS']);
function exportedRoute(bindings,item,exported,{template,isPage}) {
  const symbol=bindings.unalias(item.checker,exported);
  const declaration=symbol?.declarations?.find(node=>bindings.exportIndex(node)!==undefined);
  const index=bindings.exportIndex(declaration);
  const restrictions=[...template.restrictions,...configurationRestrictions(item.owner)];
  if(index===undefined) restrictions.push('unresolved-route-export');
  return {symbol:index===undefined?null:bindings.graph.nodes[index].id,build:item.owner,application:'next-app',framework:'next',
    kind:isPage?'navigation':'http',path:template.path,methods:isPage?[]:[exported.name],site:sourceSite(declaration||item.file),
    restrictions,order:0};
}
function implicitHead(routes,exports) {
  if(exports.some(exported=>exported.name==='HEAD')) return;
  for(const route of routes.filter(route=>route.methods.includes('GET'))) route.methods.push('HEAD');
}
function routes(bindings,item) {
  if(!hasNext(item.owner)) return [];
  const folders=appFolders(item.owner,item.file);
  const module=item.checker.getSymbolAtLocation(item.file);
  if(!folders||!module) return [];
  const template=routePath(folders);
  const isPage=/\/page\.tsx?$/.test(sourcePath(item.file));
  const eligible=exported=>isPage?exported.name==='default':HTTP.has(exported.name);
  const exports=item.checker.getExportsOfModule(module);
  const result=exports.filter(eligible).map(exported=>exportedRoute(bindings,item,exported,{template,isPage}));
  if(!isPage) implicitHead(result,exports);
  return result;
}
module.exports={routes};
