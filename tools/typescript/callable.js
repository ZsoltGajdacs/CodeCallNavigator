'use strict';
const data=require('./callable-data.js');
const syntax=require('./observer-syntax.js');
const {declarationKey,canonical}=require('./paths.js');
const {collectFacts}=require('./callable-facts.js');
const {Values}=require('./callable-values.js');
const {Returns}=require('./callable-returns.js');
const {relationshipEvidence}=require('./evidence.js');
class CallableFlow {
  constructor(bindings) {
    this.bindings=bindings;this.ts=bindings.ts;this.parameters=new Map();this.results=new WeakMap();
    this.checkers=new WeakMap(bindings.files.map(({file,checker})=>[file,checker]));
    Object.assign(this,collectFacts(bindings));
    this.values=new Values(this);this.returns=new Returns(this);
  }
  context(node) {return {owner:syntax.callable(this.ts,node)};}
  checkerFor(node,fallback) {return this.checkers.get(node.getSourceFile())||fallback;}
  owner(node) {
    const callable=syntax.callable(this.ts,node);
    return callable?this.bindings.index(callable):this.bindings.exportIndex(node.getSourceFile());
  }
  collect() {
    let changed=true;
    while(changed) {
      changed=false;
      for(const site of this.calls) changed=this.site(site)||changed;
    }
  }
  site({node,checker}) {
    if(node.arguments.some(argument=>this.ts.isSpreadElement(argument))) return false;
    const callee=this.values.resolve(checker,node.expression,this.context(node));
    if(callee.unsafe) return false;
    let changed=false;
    for(const index of callee.targets.keys()) {
      const declaration=this.bindings.byIndex.get(index);
      if(declaration) changed=this.arguments(checker,node,declaration,index)||changed;
    }
    return changed;
  }
  arguments(checker,node,declaration,index) {
    if(!declaration.parameters.every(parameter=>this.returns.simpleParameter(parameter))) return false;
    const environment=this.returns.environment(checker,node,declaration,{context:this.context(node),seen:new Set(),index});
    let changed=false;
    for(const parameter of declaration.parameters) {
      const value=environment.get(declarationKey(parameter));
      if(!value.unsafe) changed=this.mergeParameter(parameter,value)||changed;
    }
    return changed;
  }
  mergeParameter(parameter,value) {
    const key=declarationKey(parameter),previous=this.parameters.get(key);
    const next=data.merge(previous?[previous,value]:[value]);
    if(previous&&data.fingerprint(previous)===data.fingerprint(next)) return false;
    this.parameters.set(key,next);return true;
  }
  interested(checker,node) {
    const expression=syntax.unwrap(this.ts,node.expression||node.tag);
    if(this.ts.isCallExpression(expression)||this.ts.isElementAccessExpression(expression)) return true;
    return (this.bindings.symbol(checker,expression)?.declarations||[]).some(declaration=>this.bindings.context.owners.has(canonical(declaration.getSourceFile().fileName)));
  }
  result(checker,node) {
    if(this.results.has(node)) return this.results.get(node);
    const result=this.values.resolve(checker,node.expression||node.tag,this.context(node));
    this.results.set(node,result);return result;
  }
  targets(checker,node) {
    const result=this.result(checker,node);
    if(result.unsafe) return [];
    return [...result.targets].map(([index,origins])=>({index,certainty:'possible',provenance:'codecallnavigator.callable.v1',
      evidence:[...origins.values()].map(origin=>this.evidence(node,index,origin))}));
  }
  evidence(node,index,origin) {
    const evidence=relationshipEvidence(this.bindings.graph,{rule:`codecallnavigator.callable:${origin.kind}`,semantics:'possible-invocation',site:node,
      registration:origin.site,from:this.owner(node),to:index});
    const id=this.bindings.graph.nodes[origin.support]?.id;
    if(this.supports(id,origin.support,evidence)) evidence.supporting_symbols.push(id);
    return evidence;
  }
  supports(id,index,evidence) {return id&&!evidence.supporting_symbols.includes(id)&&!this.bindings.graph.excluded.has(index);}
  diagnostic(checker,node) {
    if(!this.interested(checker,node)) return undefined;
    const result=this.result(checker,node);
    return result.unsafe?`unresolved callable flow: ${result.reason}`:undefined;
  }
}
function collectCallables(bindings) {
  const flow=new CallableFlow(bindings);
  flow.collect();bindings.callables=flow;
}
module.exports={collectCallables};
