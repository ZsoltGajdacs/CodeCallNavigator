'use strict';
const {implementation}=require('./flow-function-binding.js');
const {MAX_FLOW_NODES}=require('./flow-limits.js');

function trivial(source,node) {
  const {ts,checker}=source;
  if([ts.isNumericLiteral,ts.isStringLiteral,ts.isNoSubstitutionTemplateLiteral,ts.isBigIntLiteral].some(check=>check(node))) return true;
  if([ts.SyntaxKind.TrueKeyword,ts.SyntaxKind.FalseKeyword,ts.SyntaxKind.NullKeyword].includes(node.kind)) return true;
  if(!ts.isIdentifier(node)) return false;
  const declarations=checker.getSymbolAtLocation(node)?.declarations||[];
  return declarations.length===1&&ts.isParameter(declarations[0]);
}
function orderedChildren(source,node) {
  const {ts}=source;
  if(ts.isParenthesizedExpression(node)) return [node.expression];
  if(ts.isConditionalExpression(node)) return [node.condition,node.whenTrue,node.whenFalse];
  if(ts.isBinaryExpression(node)&&[ts.SyntaxKind.AmpersandAmpersandToken,ts.SyntaxKind.BarBarToken].includes(node.operatorToken.kind)) return [node.left,node.right];
  if(implementation(source,node)) return [...node.arguments];
  return undefined;
}
function supportedArguments(source,args,ordered) {
  if(!ordered) return args.every(argument=>trivial(source,argument));
  // Proof has its own bound and cannot consume the IR budget or change old shapes.
  const pending=[...args];let visited=0;
  while(pending.length) {
    if(++visited>MAX_FLOW_NODES) return false;
    const node=pending.pop();
    if(trivial(source,node)) continue;
    const children=orderedChildren(source,node);
    if(!children) return false;
    pending.push(...children);
  }
  return true;
}
function isolatedValue(value) {
  return value.calls.length===1&&!value.regions.length&&value.span.start<value.span.end;
}
/** Called only for an actual final CallExpression Value, never generic call collection. */
function finalInvocation(source,node,value,ordered=false) {
  if(!implementation(source,node)||!supportedArguments(source,node.arguments,ordered)) return value;
  if(!isolatedValue(value)) return value;
  const call=value.calls[0];
  if(call.span.start!==value.span.start||call.span.end!==value.span.end) return value;
  call.invocation={timing:'synchronous',dispatch:'direct',isolated_ordered:true};
  return value;
}
module.exports={finalInvocation};
