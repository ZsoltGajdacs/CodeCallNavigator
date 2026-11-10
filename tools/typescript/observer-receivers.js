'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {immutableReceiver}=require('./observer-values.js');
class Receiver {
  constructor(bindings,site,channel) {this.bindings=bindings;this.site=site;this.channel=channel;this.ts=bindings.ts;}
  accept(node,seen=new Set()) {
    if(this.ts.isParenthesizedExpression(node)) return this.accept(node.expression,seen);
    if(syntax.dynamicReceiver(this.ts,node)) return this.dynamic(node);
    if(seen.has(node)) return false;
    return this.symbol(node,new Set(seen).add(node));
  }
  symbol(node,seen) {
    const declarations=this.bindings.symbol(this.site.checker,node)?.declarations||[];
    return declarations.length>0&&declarations.every(declaration=>this.declaration(declaration,seen));
  }
  declaration(node,seen) {
    if(this.site.writes.has(declarationKey(node))) return false;
    if(this.origin(node)) return true;
    if(!this.immutable(node)) return false;
    return !!node.initializer&&this.accept(node.initializer,seen);
  }
  immutable(node) {
    if(!this.ts.isVariableDeclaration(node)) return false;
    return syntax.isConst(this.ts,node);
  }
  dynamic() {return false;}
}
class SingletonReceiver extends Receiver {
  origin(node) {return declarationKey(node)===this.channel.singleton;}
  dynamic(node) {
    if(node.kind!==this.ts.SyntaxKind.ThisKeyword) return false;
    return this.lexicalThis(node);
  }
  lexicalThis(node) {
    for(let parent=node.parent;parent;parent=parent.parent) {
      if(this.ts.isArrowFunction(parent)) continue;
      if(!this.ts.isFunctionLike(parent)) continue;
      return this.member(parent);
    }
    return false;
  }
  member(node) {
    if(!this.ts.isMethodDeclaration(node)&&!this.ts.isConstructorDeclaration(node)) return false;
    return declarationKey(node.parent)===declarationKey(this.channel.node.parent);
  }
}
class StaticReceiver extends Receiver {
  origin(node) {return declarationKey(node)===declarationKey(this.channel.node.parent);}
}
function collectionReceiver(bindings,site,expression,channel) {
  if(!bindings.ts.isPropertyDeclaration(channel.node)) return true;
  if(!bindings.ts.isPropertyAccessExpression(expression)) return false;
  const resolver=channel.instance?new SingletonReceiver(bindings,site,channel):new StaticReceiver(bindings,site,channel);
  return resolver.accept(expression.expression);
}
function registrarReceiver(bindings,site,expression,registration) {
  if(!bindings.ts.isMethodDeclaration(registration.fn)) return true;
  if(!bindings.ts.isPropertyAccessExpression(expression)) return false;
  if(registration.channel.instance) return new SingletonReceiver(bindings,site,registration.channel).accept(expression.expression);
  return immutableReceiver(bindings,site.checker,expression.expression,site.writes);
}
module.exports={registrarReceiver,collectionReceiver};
