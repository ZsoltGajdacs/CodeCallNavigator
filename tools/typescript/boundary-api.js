'use strict';
const {declarationKey}=require('./paths.js');
const {isWrapper}=require('./syntax.js');
const {routerDeclaration,escaped}=require('./boundary-router.js');
const {GLOBAL_FETCH}=require('./boundary-global.js');
function unwrap(ts,node) {while(node&&isWrapper(ts,node)) node=node.expression;return node;}
const {nextApi}=require('./boundary-import.js');
function unchanged(writes,declaration) {return !writes.has(declarationKey(declaration));}
function routerInitializer(bindings,checker,node) {
  if(!node||!bindings.ts.isCallExpression(node)) return false;
  return node.arguments.length===0&&nextApi(bindings,checker,node.expression)==='useRouter';
}
function routerMethod(bindings,checker,node,writes) {
  const {ts}=bindings;
  if(!ts.isPropertyAccessExpression(node)) return undefined;
  if(!['push','replace'].includes(node.name.text)) return undefined;
  const declaration=routerDeclaration(bindings,checker,unwrap(ts,node.expression),writes);
  if(!declaration) return undefined;
  const initializer=unwrap(ts,declaration.initializer);
  if(!routerInitializer(bindings,checker,initializer)) return undefined;
  return escaped(bindings,declaration)?undefined:node.name.text;
}
function builtinFetch(bindings,checker,node,writes) {
  if(writes.has(GLOBAL_FETCH)) return false;
  if(!bindings.ts.isIdentifier(node)||node.text!=='fetch') return false;
  const declarations=bindings.symbol(checker,node)?.declarations||[];
  const standard=declaration=>unchanged(writes,declaration)&&
    /\/(?:typescript\/lib\/lib\.dom\.d\.ts|@types\/node\/.*\.d\.ts)$/.test(declaration.getSourceFile().fileName);
  return declarations.length>0&&declarations.every(standard);
}
function literal(ts,node) {node=unwrap(ts,node);return node&&ts.isStringLiteralLike(node)?node.text:undefined;}
module.exports={nextApi,routerMethod,builtinFetch,literal,unwrap};
