'use strict';
const crypto=require('node:crypto');
const {isCallable}=require('./syntax.js');

// Parsed token boundaries preserve regex/template/JSX context while excluding trivia.
function syntaxTokens(ts,node,cooked=false) {
  const tokens=[],file=node.getSourceFile();
  const visit=current=>{
    if(ts.isJSDoc(current)) return;
    const children=current.getChildren(file);
    if(children.length) {children.forEach(visit);return;}
    const text=tokenText(ts,current,file,cooked);
    if(text) tokens.push([current.kind,text]);
  };
  visit(node);
  return tokens;
}
function tokenText(ts,node,file,cooked) {
  if(cooked&&ts.isStringLiteralLike(node)) return JSON.stringify(node.text);
  if(cooked&&[ts.isIdentifier,ts.isPrivateIdentifier,ts.isNumericLiteral].some(check=>check(node))) return node.text;
  return ts.isJsxText(node)?node.text:node.getText(file);
}
function tokenFingerprint(ts,node) {
  return crypto.createHash('sha256').update(JSON.stringify(syntaxTokens(ts,node))).digest('hex');
}
function nameKey(ts,name) {
  if(ts.isStringLiteralLike(name)) return JSON.stringify(name.text);
  if([ts.isIdentifier,ts.isPrivateIdentifier,ts.isNumericLiteral].some(check=>check(name))) return name.text;
  return JSON.stringify(syntaxTokens(ts,name,true));
}
function declarationName(ts,node) {
  if(node.name) return nameKey(ts,node.name);
  const parent=node.parent;
  if(parent?.name && parent.initializer===node) return nameKey(ts,parent.name);
  if(ts.isConstructorDeclaration(node)) return '<constructor>';
  if(ts.isFunctionDeclaration(node)||ts.isClassDeclaration(node)) return '<default>';
  return undefined;
}
function role(ts,node) {
  const parent=node.parent;
  if(ts.isCallExpression(parent)||ts.isNewExpression(parent)) return `argument:${parent.arguments.indexOf(node)}`;
  if(ts.isReturnStatement(parent)) return 'return';
  return ts.SyntaxKind[parent.kind];
}
class SemanticKeys {
  constructor(bindings) {
    this.bindings=bindings;this.ts=bindings.ts;
    this.byNode=new Map();this.occurrences=new Map();
  }
  isOwner(node) {
    if(isCallable(this.ts,node)) return true;
    return [this.ts.isClassLike,this.ts.isSourceFile,this.ts.isModuleDeclaration,
      this.ts.isInterfaceDeclaration,this.ts.isObjectLiteralExpression].some(check=>check(node));
  }
  owner(node) {
    let enclosing=node.parent;
    while(enclosing&&!this.isOwner(enclosing)) enclosing=enclosing.parent;
    return enclosing&&!this.ts.isSourceFile(enclosing)?this.key(enclosing):'<module>';
  }
  base(node) {
    const owner=this.owner(node),name=declarationName(this.ts,node);
    const index=this.bindings.exportIndex(node);
    const callable=isCallable(this.ts,node)&&index!==undefined;
    const signature=callable?this.bindings.graph.nodes[index].signature:'';
    if(name) return JSON.stringify([owner,this.ts.SyntaxKind[node.kind],name,signature]);
    return JSON.stringify([owner,this.ts.SyntaxKind[node.kind],role(this.ts,node),tokenFingerprint(this.ts,node)]);
  }
  key(node) {
    if(this.byNode.has(node)) return this.byNode.get(node);
    const base=this.base(node),group=`${node.getSourceFile().fileName}:${base}`;
    const occurrence=this.occurrences.get(group)||0;
    this.occurrences.set(group,occurrence+1);
    const value=JSON.stringify([base,occurrence]);
    this.byNode.set(node,value);
    return value;
  }
}
function semanticKeys(bindings) {
  const {ts,graph}=bindings,keys={},semantic=new SemanticKeys(bindings);
  for(const {file} of bindings.files) {
    const visit=node=>{
      const index=bindings.exportIndex(node);
      if(index!==undefined) keys[graph.nodes[index].id]=ts.isSourceFile(node)?'<module>':semantic.key(node);
      ts.forEachChild(node,visit);
    };
    visit(file);
  }
  return keys;
}
module.exports={semanticKeys,tokenFingerprint};
