'use strict';
const {nullish,logicalAssignment}=require('./flow-operators.js');
/** Explicit boundaries of the source-flow model; compiler syntax, never name guesses. */
function rules(ts) {
  return [
    [node=>ts.isAwaitExpression(node),'Await may suspend and resume; this source flow does not establish runtime scheduling.'],
    [node=>ts.isForOfStatement(node)&&node.awaitModifier,'Async iteration may suspend; scheduling and iterator cleanup are not expanded.'],
    [node=>nullish(ts,node),'Nullish-coalescing internals are shown as one source step; operand calls may be skipped.'],
    [node=>logicalAssignment(ts,node),'Logical-assignment internals are shown as one source step; assignment and operand calls may be skipped.'],
    [node=>!!node.questionDotToken,'Optional-chain internals are shown as one source step; member calls may be skipped.'],
    [node=>ts.isBindingElement(node)&&node.initializer,'Destructuring defaults are not expanded into conditional initialization.'],
    [node=>ts.isYieldExpression(node),'Generator yield/resumption is not expanded; this flow is incomplete.'],
  ];
}
function diagnostics(ts,declaration) {
  const messages=new Set(),checks=rules(ts);
  function visit(node) {
    for(const [matches,message] of checks) if(matches(node)) messages.add(message);
    if(node!==declaration&&ts.isFunctionLike(node)) return;
    ts.forEachChild(node,visit);
  }
  visit(declaration);return [...messages];
}
module.exports={diagnostics};
