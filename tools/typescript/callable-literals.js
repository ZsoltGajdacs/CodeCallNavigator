'use strict';
const data=require('./callable-data.js');
const {canonical}=require('./paths.js');
function special(ts,node) {
  const flags={
    [ts.SyntaxKind.NullKeyword]:{nullish:true},
    [ts.SyntaxKind.FalseKeyword]:{falsy:true},
    [ts.SyntaxKind.TrueKeyword]:{truthy:true},
  };
  return flags[node.kind];
}
function undefinedValue(bindings,checker,node) {
  const {ts}=bindings;
  if(!ts.isIdentifier(node)||node.text!=='undefined') return undefined;
  if(!(checker.getTypeAtLocation(node).flags&ts.TypeFlags.Undefined)) return undefined;
  const declarations=bindings.symbol(checker,node)?.declarations||[];
  const owned=declarations.some(declaration=>bindings.context.owners.has(canonical(declaration.getSourceFile().fileName)));
  return owned?undefined:data.value({nullish:true});
}
function literal(bindings,checker,node) {
  const {ts}=bindings,flags=special(ts,node);
  if(flags) return data.value(flags);
  if(ts.isStringLiteralLike(node)||ts.isNumericLiteral(node)) {
    const truthy=!!(ts.isNumericLiteral(node)?Number(node.text):node.text);
    return data.value({truthy,falsy:!truthy});
  }
  return undefinedValue(bindings,checker,node);
}
module.exports={literal};
