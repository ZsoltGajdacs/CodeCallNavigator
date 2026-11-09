'use strict';
const {declarationKey}=require('./paths.js');
function isCallable(ts,node) {
  if(!ts.isFunctionLike(node)) return false;
  return !['CallSignature','ConstructSignature','FunctionType','ConstructorType'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function isCallback(ts,node) {
  return ['ArrowFunction','FunctionExpression'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function isWrapper(ts,node) {
  return ['ParenthesizedExpression','AsExpression','NonNullExpression','SatisfiesExpression','TypeAssertionExpression'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function isCallSite(ts,node) {
  return ['CallExpression','NewExpression','TaggedTemplateExpression'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function isJsxSite(ts,node) {
  return ['JsxOpeningElement','JsxSelfClosingElement'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function isAccess(ts,node) {
  return ['PropertyAccessExpression','ElementAccessExpression'].some(name=>node.kind===ts.SyntaxKind[name]);
}
function assignedName(ts,node) {
  if(['VariableDeclaration','PropertyAssignment','PropertyDeclaration'].some(name=>node.parent.kind===ts.SyntaxKind[name])) return node.parent.name.getText();
  return undefined;
}
function label(ts,node) {
  if(ts.isConstructorDeclaration(node)) return node.parent.name?.text || '<constructor>';
  if(node.name) return node.name.getText();
  const assigned=assignedName(ts,node);
  if(assigned) return assigned;
  if(ts.isFunctionDeclaration(node)) return 'default';
  const point=node.getSourceFile().getLineAndCharacterOfPosition(node.getStart());
  return `<callback:${point.line+1}:${point.character+1}>`;
}
module.exports={isCallable,isCallback,isWrapper,isCallSite,isJsxSite,isAccess,label,declarationKey};
