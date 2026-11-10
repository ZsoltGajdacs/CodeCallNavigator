'use strict';
const data=require('./callable-data.js');
const nullishRule={retain:left=>({...left,nullish:false}),fallback:left=>left.nullish};
const orRule={retain:left=>({...left,nullish:false,falsy:false}),fallback:left=>left.nullish||left.falsy};
function rule(ts,kind) {
  if(kind===ts.SyntaxKind.QuestionQuestionToken) return nullishRule;
  if(kind===ts.SyntaxKind.BarBarToken) return orRule;
  return undefined;
}
function logical(values,checker,node,{context,seen}) {
  const operation=rule(values.ts,node.operatorToken.kind);
  if(!operation) return data.unknown('unsupported callable expression');
  const left=values.resolve(checker,node.left,context,seen);
  if(left.unsafe) return left;
  const branches=[operation.retain(left)];
  if(operation.fallback(left)) {
    const right=values.resolve(checker,node.right,context,seen);
    if(right.unsafe) return right;
    branches.push(right);
  }
  return data.derived(data.merge(branches),{kind:'logical',site:node});
}
module.exports={logical};
