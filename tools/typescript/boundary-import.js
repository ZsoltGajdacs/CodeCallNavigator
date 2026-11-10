'use strict';
function importDeclaration(ts,declaration) {
  for(let current=declaration;current;current=current.parent) {
    if(ts.isImportDeclaration(current)) return current;
  }
  return undefined;
}
function importModule(bindings,checker,node) {
  const reference=bindings.ts.isPropertyAccessExpression(node)?node.expression:node;
  const symbol=checker.getSymbolAtLocation(reference);
  const declaration=symbol?.declarations?.[0];
  return importDeclaration(bindings.ts,declaration)?.moduleSpecifier?.text;
}
function packageSymbol(bindings,checker,node) {
  const symbol=bindings.symbol(checker,node);
  const declarations=symbol?.declarations||[];
  if(!declarations.length) return undefined;
  return declarations.every(item=>/\/node_modules\/next\//.test(item.getSourceFile().fileName))?symbol:undefined;
}
function allowed(module,name) {
  if(module==='next/navigation') return ['useRouter','redirect','permanentRedirect'].includes(name);
  return module==='next/link'&&['default','Link'].includes(name);
}
function nextApi(bindings,checker,node) {
  const module=importModule(bindings,checker,node);
  const symbol=packageSymbol(bindings,checker,node);
  if(!symbol) return undefined;
  return allowed(module,symbol.name)?symbol.name:undefined;
}
module.exports={nextApi};
