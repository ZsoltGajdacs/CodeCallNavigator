'use strict';
// Access belongs to class declarations, never to a function's lexical scope.
function classCallable(ts, declaration) {
  const kinds = ['MethodDeclaration', 'GetAccessor', 'SetAccessor', 'Constructor'];
  return kinds.some(kind => declaration.kind === ts.SyntaxKind[kind]) && ts.isClassLike(declaration.parent);
}
function methodVisibility(ts, declaration) {
  if (!classCallable(ts, declaration)) return undefined;
  if (declaration.name && ts.isPrivateIdentifier(declaration.name)) return 'private';
  const flags = ts.getCombinedModifierFlags(declaration);
  if (flags & ts.ModifierFlags.Private) return 'private';
  if (flags & ts.ModifierFlags.Protected) return 'protected';
  return 'public';
}
module.exports = { methodVisibility };
