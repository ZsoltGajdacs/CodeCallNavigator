'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
class ContextEscapes {
  constructor(bindings,uses) {this.bindings=bindings;this.uses=uses;this.ts=bindings.ts;}
  value(node) {
    return [this.ts.isIdentifier,this.ts.isPropertyAccessExpression,this.ts.isElementAccessExpression,
      this.ts.isObjectLiteralExpression,this.ts.isArrayLiteralExpression,this.ts.isNewExpression].some(check=>check(node));
  }
  same(source,target) {
    if(source===target) return true;
    const key=syntax.symbolKey(source.getSymbol());
    return !!key&&key===syntax.symbolKey(target.getSymbol());
  }
  inspect(node,checker) {
    if(!this.value(node)) return;
    const target=checker.getContextualType(node);
    if(!target) return;
    const source=checker.getApparentType(checker.getTypeAtLocation(node));
    if(!(source.flags&this.ts.TypeFlags.Object)) return;
    if(checker.getSignaturesOfType(source,this.ts.SignatureKind.Call).length) return;
    if(this.same(source,target)) return;
    this.escape(source);
  }
  escape(type) {
    for(const symbol of type.getProperties()) this.method(symbol);
    for(const channel of this.uses.typedChannels(type,true)) {
      if(channel.valid) this.bindings.graph.diagnostic('unresolved observer receiver type-changing escape');
      channel.valid=false;
    }
  }
  method(symbol) {
    for(const declaration of symbol.declarations||[]) {
      if(this.bindings.index(declaration)!==undefined) this.uses.writes.add(declarationKey(declaration));
    }
  }
}
module.exports={ContextEscapes};
