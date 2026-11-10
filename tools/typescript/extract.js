'use strict';
const {canonical}=require('./paths.js');
const {architectureFacts}=require('./architecture-facts.js');
const {flowInventory}=require('./flow-export.js');
const {accessorCalls}=require('./accessors.js');
const {Graph}=require('./graph.js');
const {Bindings}=require('./bindings.js');
const {isCallable,isCallSite,isJsxSite,isAccess}=require('./syntax.js');
const {reactImport}=require('./react-import.js');
const {ReactRules}=require('./react.js');
const {markNextEntries}=require('./next.js');
const {semanticKeys}=require('./semantic.js');
const {collectObservers}=require('./observer.js');
function argumentReferences(bindings,checker,node,from) {
  const targets=(node.arguments||[]).flatMap(argument=>bindings.resolve(checker,argument));
  for(const target of targets) bindings.graph.edge(from,target,'reference',{certainty:'possible',provenance:'typescript.compiler:argument-reference'});
}
function reportUnresolvedCall(bindings,checker,node) {
  const declaration=checker.getResolvedSignature(node)?.declaration;
  if(!declaration || bindings.context.owners.has(canonical(declaration.getSourceFile().fileName))) bindings.graph.diagnostic('unresolved dynamic call sites');
}
function ordinaryCall(bindings,checker,node,from) {
  const {ts,graph}=bindings;
  argumentReferences(bindings,checker,node,from);
  const targets=bindings.callTargets(checker,node);
  for(const {index,...options} of targets) graph.edge(from,index,ts.isNewExpression(node)?'construct':'call',options);
  if(!targets.length) reportUnresolvedCall(bindings,checker,node);
}
function visitSite(bindings,react,{checker,node,current}) {
  if(isCallSite(bindings.ts,node)) ordinaryCall(bindings,checker,node,current);
  if(bindings.ts.isCallExpression(node)) react.call(checker,node,current);
  if(isJsxSite(bindings.ts,node)) react.jsx(checker,node,current);
  if(isAccess(bindings.ts,node)) accessorCalls(bindings,checker,node,current);
}
function walkFile(bindings,react,{file,checker,module}) {
  function visit(node,from) {
    const scoped=isCallable(bindings.ts,node)||bindings.ts.isClassStaticBlockDeclaration(node);
    const current=scoped?bindings.index(node):from;
    visitSite(bindings,react,{checker,node,current});
    const field=bindings.ts.isPropertyDeclaration(node);
    bindings.ts.forEachChild(node,child=>{
      const owner=field&&child===node.initializer ? bindings.index(node) : current;
      visit(child,owner);
    });
  }
  visit(file,module);
}
function extract(context) {
  const graph=new Graph(context);
  const bindings=new Bindings(context,graph);
  bindings.collect();
  const react=new ReactRules(bindings,graph,reactImport);
  react.collectProps();
  collectObservers(bindings);
  for(const file of bindings.files) {
    walkFile(bindings,react,file);
    markNextEntries(context,bindings,graph,file);
  }
  return {...graph.result(),export_identities:semanticKeys(bindings),flow_inventory:flowInventory(bindings),architecture:{version:1,files:bindings.files.map(file=>architectureFacts(context,file))}};
}
module.exports={extract};
