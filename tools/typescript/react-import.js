'use strict';
function importedName(ts,declaration,expression) {
  let current=declaration;
  while(current && !ts.isImportDeclaration(current)) current=current.parent;
  if(current?.moduleSpecifier.text!=='react') return undefined;
  return ts.isImportSpecifier(declaration) ? (declaration.propertyName||declaration.name).text : expression.name?.text;
}
function reactImport(ts,checker,expression) {
  const name=ts.isPropertyAccessExpression(expression) ? expression.name : expression;
  const symbol=checker.getSymbolAtLocation(name);
  if(!symbol) return undefined;
  const imports=(symbol.declarations||[]).map(declaration=>importedName(ts,declaration,expression));
  const imported=imports.find(Boolean);
  if(imported) return imported;
  const resolved=symbol.flags & ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol;
  const reactDeclaration=(resolved.declarations||[]).some(declaration=>/\/node_modules\/(?:@types\/react|react)\//.test(declaration.getSourceFile().fileName));
  return reactDeclaration ? resolved.name : undefined;
}
module.exports={reactImport};
