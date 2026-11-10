'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
class Values {
  constructor(bindings,checker,writes) {this.bindings=bindings;this.checker=checker;this.writes=writes;this.ts=bindings.ts;}
  accept(node,seen=new Set()) {
    node=syntax.unwrap(this.ts,node);
    if(!node||seen.has(node)) return false;
    seen=new Set(seen).add(node);
    if([this.ts.isArrowFunction,this.ts.isFunctionExpression].some(check=>check(node))) return true;
    if(this.ts.isConditionalExpression(node)) return this.conditional(node,seen);
    return this.symbol(node,seen);
  }
  conditional(node,seen) {return this.accept(node.whenTrue,seen)&&this.accept(node.whenFalse,seen);}
  symbol(node,seen) {
    if(this.ts.isPropertyAccessExpression(node)&&!this.receiver(node.expression)) return false;
    const declarations=this.bindings.symbol(this.checker,node)?.declarations||[];
    return declarations.length>0&&declarations.every(declaration=>this.declaration(declaration,seen));
  }
  receiver(node,seen=new Set()) {
    if(syntax.dynamicReceiver(this.ts,node)) return false;
    if(this.ts.isParenthesizedExpression(node)) return this.receiver(node.expression,seen);
    if(seen.has(node)) return false;
    seen=new Set(seen).add(node);
    const type=this.checker.getTypeAtLocation(node);
    if(type.flags&(this.ts.TypeFlags.Any|this.ts.TypeFlags.Unknown)) return false;
    const declarations=this.bindings.symbol(this.checker,node)?.declarations||[];
    return declarations.length>0&&declarations.every(declaration=>this.receiverDeclaration(declaration,seen));
  }
  receiverDeclaration(node,seen) {
    if(this.writes.has(declarationKey(node))) return false;
    if(syntax.decoratedOwner(this.ts,node)) return false;
    if(this.ts.isClassDeclaration(node)) return true;
    if(!this.immutableVariable(node)||!node.initializer) return false;
    return this.receiverValue(node.initializer,seen);
  }
  receiverValue(node,seen) {
    if(this.ts.isObjectLiteralExpression(node)||this.ts.isClassExpression(node)) return true;
    return this.receiver(node,seen);
  }
  declaration(node,seen) {
    if(this.writes.has(declarationKey(node))) return false;
    if(syntax.decoratedOwner(this.ts,node)) return false;
    if(this.ts.isFunctionDeclaration(node)||this.ts.isMethodDeclaration(node)) return true;
    if(!this.immutableVariable(node)) return false;
    return !!node.initializer&&this.accept(node.initializer,seen);
  }
  immutableVariable(node) {
    if(!this.ts.isVariableDeclaration(node)) return false;
    return syntax.isConst(this.ts,node)&&this.ts.isIdentifier(node.name);
  }
}
function stableTargets(bindings,checker,node,writes) {
  if(!new Values(bindings,checker,writes).accept(node)) return [];
  return bindings.resolve(checker,node).filter(index=>!bindings.graph.excluded.has(index));
}
function immutableReceiver(bindings,checker,node,writes) {return new Values(bindings,checker,writes).receiver(node);}
function assignmentContainer(ts,node) {
  const parent=node.parent;
  if([ts.isArrayLiteralExpression,ts.isObjectLiteralExpression,ts.isSpreadElement].some(check=>check(parent))) return true;
  if(ts.isPropertyAssignment(parent)) return parent.initializer===node;
  if(ts.isShorthandPropertyAssignment(parent)) return parent.name===node;
  return parent.expression===node&&syntax.unwrap(ts,parent)!==parent;
}
function assignment(ts,node) {
  const parent=node.parent;
  if(!ts.isBinaryExpression(parent)||parent.left!==node) return false;
  return parent.operatorToken.kind>=ts.SyntaxKind.FirstAssignment&&parent.operatorToken.kind<=ts.SyntaxKind.LastAssignment;
}
function iterationWrite(ts,node) {
  const parent=node.parent;
  return [ts.isForOfStatement,ts.isForInStatement].some(check=>check(parent))&&parent.initializer===node;
}
function update(ts,node) {
  const parent=node.parent;
  if(!ts.isPrefixUnaryExpression(parent)&&!ts.isPostfixUnaryExpression(parent)) return false;
  return [ts.SyntaxKind.PlusPlusToken,ts.SyntaxKind.MinusMinusToken].includes(parent.operator);
}
function written(ts,node) {
  while(assignmentContainer(ts,node)) node=node.parent;
  return [assignment,iterationWrite,update].some(check=>check(ts,node))||ts.isDeleteExpression(node.parent);
}
function parameterUnchanged(bindings,checker,parameter,body) {
  const {ts}=bindings,key=declarationKey(parameter);
  let valid=true;
  const matches=node=>ts.isIdentifier(node)&&node.text===parameter.name.text&&syntax.keys(bindings,checker,node).includes(key);
  const visit=node=>{
    if(matches(node)&&written(ts,node)) valid=false;
    ts.forEachChild(node,visit);
  };
  visit(body);
  return valid;
}
module.exports={stableTargets,immutableReceiver,parameterUnchanged,written};
