'use strict';
const path=require('node:path');
const {sourcePath}=require('./paths.js');
function addNames(values,name) {
  if(!name) return;
  values.add(name);
  values.add(name.split('.').at(-1));
}
function decorators(ts,checker,node) {
  const result=new Set();
  for(const decorator of ts.canHaveDecorators(node)?ts.getDecorators(node)||[]:[]) {
    const expression=ts.isCallExpression(decorator.expression)?decorator.expression.expression:decorator.expression;
    let symbol=checker.getSymbolAtLocation(expression);
    if(symbol && symbol.flags&ts.SymbolFlags.Alias) symbol=checker.getAliasedSymbol(symbol);
    if(symbol) {
      addNames(result,symbol.getName());
      addNames(result,checker.getFullyQualifiedName(symbol).replace(/^"[^"]+"\./,''));
    }
  }
  return [...result].sort();
}
function declarationKind(ts,node) {
  if(ts.isClassLike(node)) return 'class';
  if(ts.isInterfaceDeclaration(node)) return 'interface';
  if(ts.isEnumDeclaration(node)) return 'enum';
  if(ts.isTypeAliasDeclaration(node)) return 'type';
  return undefined;
}
class FileInventory {
  constructor(context,checker) {
    this.ts=context.ts; this.checker=checker; this.visitedBases=new Set();
    for(const key of ['names','kinds','annotations','method_annotations','base_types','annotated_interfaces','namespaces']) this[key]=new Set();
  }
  collect(node) {
    this.declaration(node);
    if(this.ts.isModuleDeclaration(node)) this.namespaces.add(node.name.text);
    if(this.method(node)) for(const name of decorators(this.ts,this.checker,node)) this.method_annotations.add(name);
    this.ts.forEachChild(node,child=>this.collect(child));
  }
  method(node) {
    return this.ts.isMethodDeclaration(node)||this.ts.isGetAccessorDeclaration(node)||this.ts.isSetAccessorDeclaration(node);
  }
  declaration(node) {
    const kind=declarationKind(this.ts,node);
    if(!kind) return;
    this.kinds.add(kind);
    addNames(this.names,node.name?.text);
    for(const name of decorators(this.ts,this.checker,node)) this.annotations.add(name);
    const bases=(node.heritageClauses||[]).flatMap(clause=>[...clause.types]);
    for(const base of bases) this.base(base);
  }
  base(node) {
    this.baseType(this.checker.getTypeAtLocation(node));
  }
  baseType(type) {
    const symbol=type.getSymbol()||type.aliasSymbol;
    if(!symbol || this.visitedBases.has(type)) return;
    this.visitedBases.add(type);
    addNames(this.base_types,symbol.getName());
    this.baseMembers(type,symbol);
  }
  baseMembers(type,symbol) {
    for(const declaration of symbol.declarations||[]) this.interfaceAnnotations(declaration);
    for(const base of type.getBaseTypes?.()||[]) this.baseType(base);
  }
  interfaceAnnotations(node) {
    if(!this.ts.isInterfaceDeclaration(node)) return;
    for(const name of decorators(this.ts,this.checker,node)) this.annotated_interfaces.add(name);
  }
  values() {
    const keys=['names','kinds','annotations','method_annotations','base_types','annotated_interfaces','namespaces'];
    return Object.fromEntries(keys.map(key=>[key,[...this[key]].sort()]));
  }
}
/** Compiler-owned source facts only; matching and graph inclusion live elsewhere. */
function architectureFacts(context,{file,checker,owner}) {
  const inventory=new FileInventory(context,checker);
  inventory.collect(file);
  const source=sourcePath(file);
  const relative=path.relative(path.dirname(owner),source);
  return {build:owner,project:path.basename(path.dirname(owner)),file:source,language:'typescript',
    ...inventory.values(),is_interface:inventory.kinds.has('interface'),content:file.text,
    is_test:/(?:^|\/)(?:__tests__|tests?|test-support|test-fixtures)(?:\/|$)|\.(?:test|spec)\.[cm]?tsx?$/.test(relative)};
}
module.exports={architectureFacts};
