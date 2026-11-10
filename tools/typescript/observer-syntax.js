'use strict';
const {declarationKey}=require('./paths.js');
const {isCallable,isWrapper}=require('./syntax.js');
function unwrap(ts,node) {
  while(node&&isWrapper(ts,node)) node=node.expression;
  return node;
}
function keys(bindings,checker,node) {
  const {ts}=bindings;
  const shorthand=ts.isShorthandPropertyAssignment(node.parent)&&node.parent.name===node?checker.getShorthandAssignmentValueSymbol(node.parent):undefined;
  return ((bindings.unalias(checker,shorthand)||bindings.symbol(checker,node))?.declarations||[]).map(declarationKey);
}
function symbolKey(symbol) {return symbol?.declarations?.[0]&&declarationKey(symbol.declarations[0]);}
function dynamicReceiver(ts,node) {return [ts.SyntaxKind.ThisKeyword,ts.SyntaxKind.SuperKeyword].includes(node.kind);}
function hasModifier(ts,node,kind) {return !!ts.getModifiers(node)?.some(modifier=>modifier.kind===kind);}
function isConst(ts,declaration) {return ts.isVariableDeclarationList(declaration.parent)&&!!(declaration.parent.flags&ts.NodeFlags.Const);}
function walkOwned(bindings,visit) {
  for(const entry of bindings.files) {
    const walk=node=>{visit(node,entry);bindings.ts.forEachChild(node,walk);};
    walk(entry.file);
  }
}
function callable(ts,node) {
  for(let parent=node.parent;parent;parent=parent.parent) if(isCallable(ts,parent)) return parent;
  return undefined;
}
function owner(bindings,node) {return bindings.index(callable(bindings.ts,node)||node.getSourceFile());}
function librarySymbol(bindings,symbol) {
  return !!symbol.declarations?.length&&symbol.declarations.every(declaration=>bindings.context.programs.some(({program})=>program.isSourceFileDefaultLibrary(declaration.getSourceFile())));
}
function builtin(bindings,checker,node,name) {
  const symbol=bindings.symbol(checker,node);
  return symbol?.name===name&&librarySymbol(bindings,symbol);
}
function defaultSet(bindings,checker,node) {
  const {ts}=bindings;
  return node&&ts.isNewExpression(node)&&ts.isIdentifier(node.expression)&&!node.arguments?.length&&builtin(bindings,checker,node.expression,'Set');
}
function typePosition(ts,node) {
  for(let parent=node.parent;parent;parent=parent.parent) {
    if(ts.isTypeNode(parent)) return true;
    if(ts.isStatement(parent)||isCallable(ts,parent)) return false;
  }
  return false;
}
function decorated(ts,node) {
  if(!ts.canHaveDecorators(node)) return false;
  return (ts.getDecorators(node)?.length||0)>0;
}
function decoratedOwner(ts,node) {
  for(let parent=node;parent&&!ts.isSourceFile(parent);parent=parent.parent) if(decorated(ts,parent)) return true;
  return false;
}
function label(ts,node) {
  const parts=[node.name.getText()];
  for(let parent=node.parent;parent&&!ts.isSourceFile(parent);parent=parent.parent) {
    if(ts.isClassDeclaration(parent)||ts.isModuleDeclaration(parent)) parts.unshift(parent.name?.getText()||'<anonymous>');
  }
  return `Set:${parts.join('.')}`;
}
module.exports={unwrap,keys,symbolKey,dynamicReceiver,hasModifier,isConst,walkOwned,callable,owner,librarySymbol,builtin,defaultSet,typePosition,decoratedOwner,label};
