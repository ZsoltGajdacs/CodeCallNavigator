'use strict';
function nullish(ts,node) {return ts.isBinaryExpression(node)&&node.operatorToken.kind===ts.SyntaxKind.QuestionQuestionToken;}
function logicalAssignment(ts,node) {
  if(!ts.isBinaryExpression(node)) return false;
  return ['AmpersandAmpersandEqualsToken','BarBarEqualsToken','QuestionQuestionEqualsToken'].some(name=>node.operatorToken.kind===ts.SyntaxKind[name]);
}
function opaqueExpression(ts,node) {return ts.isOptionalChain(node)||nullish(ts,node)||logicalAssignment(ts,node);}
module.exports={nullish,logicalAssignment,opaqueExpression};
