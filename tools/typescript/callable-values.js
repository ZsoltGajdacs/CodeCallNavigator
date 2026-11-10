'use strict';
const data=require('./callable-data.js');
const syntax=require('./observer-syntax.js');
const {declarationKey}=require('./paths.js');
const {isCallable}=require('./syntax.js');
const facts=require('./callable-facts.js');
const {literal}=require('./callable-literals.js');
const {logical}=require('./callable-logical.js');
class Values {
  constructor(flow) {this.flow=flow;this.bindings=flow.bindings;this.ts=flow.bindings.ts;}
  resolve(checker,node,context,seen=new Set()) {
    node=syntax.unwrap(this.ts,node);
    if(!node||seen.has(node)) return data.unknown('recursive callable value');
    seen=new Set(seen).add(node);
    const constant=literal(this.bindings,checker,node);
    if(constant) return constant;
    if(isCallable(this.ts,node)) return this.callable(checker,node,context);
    const reader=this.reader(node);
    return reader?this[reader](checker,node,context,seen):data.unknown('unsupported callable property or dynamic key');
  }
  reader(node) {
    const {SyntaxKind:k}=this.ts;
    return {[k.ConditionalExpression]:'conditional',[k.BinaryExpression]:'logical',[k.CallExpression]:'returned',[k.Identifier]:'symbol'}[node.kind];
  }
  returned(checker,node,context,seen) {return this.flow.returns.resolve(checker,node,context,seen);}
  callable(checker,node,context) {
    if(!facts.unchanged(this.bindings,this.flow.writes,node)) return data.unknown('mutable or decorated callable declaration');
    if(facts.capturesFactory(this.bindings,checker,node,context.returning)) return data.unknown('returned closure captures factory locals');
    const index=this.bindings.index(node);
    if(!this.available(node,index)) return data.unknown('callable body is unavailable');
    return data.target(index);
  }
  available(node,index) {return index!==undefined&&!!node.body&&!this.bindings.graph.excluded.has(index);}
  symbol(checker,node,context,seen) {
    const declarations=this.bindings.symbol(checker,node)?.declarations||[];
    const usable=declarations.filter(declaration=>!isCallable(this.ts,declaration)||declaration.body);
    if(!usable.length) return data.unknown('unsupported callable declaration');
    const values=usable.map(declaration=>this.declaration(checker,declaration,context,seen));
    if(values.some(value=>value.unsafe)) return values.find(value=>value.unsafe);
    return data.merge(values);
  }
  declaration(checker,node,context,seen) {
    if(!facts.unchanged(this.bindings,this.flow.writes,node)) return data.unknown('mutable or decorated callable binding');
    if(isCallable(this.ts,node)) return this.callable(checker,node,context);
    if(this.ts.isParameter(node)) return this.parameter(node,context);
    if(!this.immutableVariable(node)) {
      return data.unknown('mutable callable alias or property');
    }
    return node.initializer?this.resolve(checker,node.initializer,context,seen):data.unknown('callable initializer unavailable');
  }
  immutableVariable(node) {
    if(!this.ts.isVariableDeclaration(node)) return false;
    return syntax.isConst(this.ts,node)&&this.ts.isIdentifier(node.name);
  }
  parameter(node,context) {
    const key=declarationKey(node);
    if(context.environment?.has(key)) return context.environment.get(key);
    if(context.returning||node.parent!==context.owner) return data.unknown('captured or unresolved callable parameter');
    return this.flow.parameters.get(key)||data.unknown('unresolved callable parameter');
  }
  conditional(checker,node,context,seen) {
    const values=[node.whenTrue,node.whenFalse].map(branch=>this.resolve(checker,branch,context,seen));
    return values.some(value=>value.unsafe)?values.find(value=>value.unsafe):data.merge(values);
  }
  logical(checker,node,context,seen) {
    return logical(this,checker,node,{context,seen});
  }
}
module.exports={Values};
