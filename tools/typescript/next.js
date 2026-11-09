'use strict';
const fs=require('node:fs');
const path=require('node:path');
const {createRequire}=require('node:module');
const COMPONENT_FILES=new Set(['page','layout','template','loading','error','global-error','not-found','default','forbidden','unauthorized']);
const ROUTE_METHODS=new Set(['GET','POST','PUT','PATCH','DELETE','HEAD','OPTIONS']);
function hasNext(owner) {
  try { return createRequire(path.join(path.dirname(owner),'__codegraph__.js')).resolve('next/package.json'); }
  catch { return undefined; }
}
function directive(ts,node,text) {
  for(const statement of node?.statements||[]) {
    if(!ts.isExpressionStatement(statement)) return false;
    if(!ts.isStringLiteral(statement.expression)) return false;
    if(statement.expression.text===text) return true;
  }
  return false;
}
function appStem(owner,file) {
  const root=path.dirname(owner);
  const relative=path.relative(root,file.fileName).replaceAll(path.sep,'/');
  const match=/^(src\/)?app\/(.*)\.tsx?$/.exec(relative);
  if(!match) return undefined;
  if(match[1] && fs.existsSync(path.join(root,'app'))) return undefined;
  if(match[2].split('/').some(segment=>segment.startsWith('_'))) return undefined;
  return path.posix.basename(match[2]);
}
function convention(stem,name) {
  if(!stem) return false;
  if(name==='default') return COMPONENT_FILES.has(stem);
  if(ROUTE_METHODS.has(name)) return stem==='route';
  if(['generateMetadata','generateViewport'].includes(name)) return ['page','layout'].includes(stem);
  if(name==='generateStaticParams') return ['page','layout','route'].includes(stem);
  return false;
}
function markDeclaration(state,declaration) {
  const {context,bindings,graph,checker}=state;
  const action=state.moduleAction || directive(context.ts,declaration.body,'use server');
  if(!state.convention && !action) return;
  const targets=bindings.resolve(checker,declaration.name||declaration.initializer||declaration);
  for(const target of targets) graph.framework.add(target);
  if(!targets.length) graph.diagnostic('unresolved Next.js framework entry bindings');
}
function markNextEntries(context,bindings,graph,{file,checker,owner}) {
  if(!hasNext(owner)) return;
  const moduleSymbol=checker.getSymbolAtLocation(file);
  if(!moduleSymbol) return;
  const stem=appStem(owner,file);
  const moduleAction=directive(context.ts,file,'use server');
  for(const exported of checker.getExportsOfModule(moduleSymbol)) {
    const symbol=bindings.unalias(checker,exported);
    const state={context,bindings,graph,checker,moduleAction,convention:convention(stem,exported.name)};
    for(const declaration of symbol?.declarations||[]) markDeclaration(state,declaration);
  }
}
module.exports={markNextEntries};
