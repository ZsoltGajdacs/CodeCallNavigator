'use strict';
function binaryAccessMode(ts,parent,node) {
  if(!ts.isBinaryExpression(parent)) return 'read';
  if(parent.left!==node) return 'read';
  if(parent.operatorToken.kind===ts.SyntaxKind.EqualsToken) return 'write';
  const kind=parent.operatorToken.kind;
  const compound=kind>=ts.SyntaxKind.FirstCompoundAssignment && kind<=ts.SyntaxKind.LastCompoundAssignment;
  return compound?'both':'read';
}
function accessMode(ts,node) {
  const parent=node.parent;
  const unary=['PostfixUnaryExpression','PrefixUnaryExpression'].some(name=>parent.kind===ts.SyntaxKind[name]);
  if(!unary) return binaryAccessMode(ts,parent,node);
  const update=['PlusPlusToken','MinusMinusToken'].some(name=>parent.operator===ts.SyntaxKind[name]);
  return update?'both':'read';
}
function accessorCalls(bindings,checker,node,from) {
  const {ts,graph}=bindings;
  const symbol=bindings.symbol(checker,node);
  const mode=accessMode(ts,node);
  for(const declaration of symbol?.declarations||[]) {
    if(ts.isGetAccessorDeclaration(declaration)&&mode!=='write') graph.edge(from,bindings.index(declaration),'accessor');
    if(ts.isSetAccessorDeclaration(declaration)&&mode!=='read') graph.edge(from,bindings.index(declaration),'accessor');
  }
}
module.exports={accessorCalls};
