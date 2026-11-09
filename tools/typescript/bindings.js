'use strict';
const path=require('node:path');
const {canonical,declarationKey}=require('./paths.js');
const {isCallable,isWrapper,label}=require('./syntax.js');
const {reactImport}=require('./react-import.js');
const {callableKind,objectMember,initializerScope}=require('./source-facts.js');
const {excludedMethod}=require('./method-policy.js');
class Bindings {
  constructor(context,graph) {
    this.context=context; this.ts=context.ts; this.graph=graph;
    this.declarations=new Map(); this.files=[]; this.props=new Map(); this.byIndex=new Map();
  }
  collect() {
    const seen=new Set();
    for(const {program} of this.context.programs) {
      const checker=program.getTypeChecker();
      for(const file of program.getSourceFiles()) this.collectFile(file,checker,seen);
    }
  }
  collectFile(file,checker,seen) {
    const source=canonical(file.fileName);
    const owner=this.context.owners.get(source);
    if(!owner || seen.has(source)) return;
    seen.add(source);
    const module=this.graph.node(file,owner,'module',{label:`<module:${path.basename(source)}>`,checker});
    this.declarations.set(declarationKey(file),module);
    const visit=node=>{this.collectDeclaration(node,owner,checker);this.ts.forEachChild(node,visit);};
    visit(file);
    this.files.push({file,checker,module,owner});
  }
  collectDeclaration(node,owner,checker) {
    if(isCallable(this.ts,node)) this.collectCallable(node,owner,checker);
    if(initializerScope(this.ts,node)) {
      this.registerDeclaration(node,owner,checker,{kind:'initializer',label:'<initializer>'});
    }
    if(!this.ts.isClassLike(node)) return;
    if(node.members.some(member=>this.ts.isConstructorDeclaration(member))) return;
    this.registerDeclaration(node,owner,checker,{kind:'implicit-constructor',label:node.name?.text || '<class>'});
  }
  collectCallable(node,owner,checker) {
    const facts={kind:callableKind(this.ts,node),label:label(this.ts,node),objectMember:objectMember(this.ts,checker,node)};
    const index=this.registerDeclaration(node,owner,checker,facts);
    this.byIndex.set(index,node);
  }
  registerDeclaration(node,owner,checker,facts) {
    const index=this.graph.node(node,owner,facts.kind,{label:facts.label,checker});
    this.declarations.set(declarationKey(node),index);
    if(excludedMethod(facts)) this.graph.excluded.add(index);
    return index;
  }
  index(node) { return node && this.declarations.get(declarationKey(node)); }
  unalias(checker,symbol) { return symbol && (symbol.flags & this.ts.SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol); }
  symbol(checker,node) {
    const symbol=checker.getSymbolAtLocation(node);
    if(symbol) return this.unalias(checker,symbol);
    if(!this.ts.isElementAccessExpression(node)) return undefined;
    const argument=node.argumentExpression;
    if(!this.ts.isStringLiteralLike(argument)&&!this.ts.isNumericLiteral(argument)) return undefined;
    return checker.getTypeAtLocation(node.expression).getProperty(argument.text);
  }
  propertyBindings(checker,node) {
    if(!this.ts.isPropertyAccessExpression(node)) return [];
    const object=this.symbol(checker,node.expression);
    return (object?.declarations||[]).flatMap(declaration=>[...(this.props.get(`${declarationKey(declaration)}:${node.name.text}`)||[])]);
  }
  declarationTargets(checker,declaration,seen) {
    const values=[...(this.props.get(declarationKey(declaration))||[])];
    const index=this.index(declaration);
    if(index!==undefined && declaration.body) return [...values,index];
    if(declaration.initializer) return [...values,...this.resolve(checker,declaration.initializer,seen)];
    if(this.ts.isClassLike(declaration)) return [...values,...this.constructorTargets(declaration)];
    return values;
  }
  constructorTargets(declaration) {
    const constructor=declaration.members.find(member=>this.ts.isConstructorDeclaration(member));
    const target=this.index(constructor||declaration);
    return target===undefined?[]:[target];
  }
  isReactWrapper(checker,node) {
    if(!this.ts.isCallExpression(node)) return false;
    return ['memo','forwardRef','useCallback'].includes(reactImport(this.ts,checker,node.expression));
  }
  conditionalTargets(checker,node,seen) {
    return [...new Set([...this.resolve(checker,node.whenTrue,seen),...this.resolve(checker,node.whenFalse,seen)])];
  }
  symbolTargets(checker,node,seen) {
    const symbol=this.symbol(checker,node);
    return (symbol?.declarations||[]).flatMap(declaration=>this.declarationTargets(checker,declaration,seen));
  }
  resolve(checker,node,seen=new Set()) {
    if(!node || seen.has(node)) return [];
    seen=new Set(seen).add(node);
    const index=this.index(node);
    if(index!==undefined) return [index];
    if(isWrapper(this.ts,node)) return this.resolve(checker,node.expression,seen);
    if(this.ts.isConditionalExpression(node)) return this.conditionalTargets(checker,node,seen);
    if(this.isReactWrapper(checker,node)) return this.resolve(checker,node.arguments[0],seen);
    const properties=this.propertyBindings(checker,node);
    if(properties.length) return [...new Set(properties)];
    return [...new Set(this.symbolTargets(checker,node,seen))];
  }
  propagatedCall(checker,node,seen=new Set()) {
    if(!node || seen.has(node)) return false;
    seen=new Set(seen).add(node);
    if(isWrapper(this.ts,node)) return this.propagatedCall(checker,node.expression,seen);
    if(this.ts.isConditionalExpression(node)) return true;
    if(this.propertyBindings(checker,node).length) return true;
    const declarations=this.symbol(checker,node)?.declarations||[];
    return declarations.some(declaration=>this.propagatedDeclaration(checker,declaration,seen));
  }
  propagatedDeclaration(checker,declaration,seen) {
    if(this.props.has(declarationKey(declaration))) return true;
    return declaration.initializer ? this.propagatedCall(checker,declaration.initializer,seen) : false;
  }
  targets(checker,node) {
    const targets=this.resolve(checker,node.expression||node.tag);
    return targets;
  }
}
module.exports={Bindings};
