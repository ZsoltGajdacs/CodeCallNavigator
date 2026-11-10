'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {collectChannels}=require('./observer-channels.js');
const {collectUses,topExpression}=require('./observer-uses.js');
const {collectRegistrations}=require('./observer-registration.js');
const {relationshipEvidence}=require('./evidence.js');
const {validateStorage}=require('./observer-intrinsics.js');
const {validateDispatchers}=require('./observer-dispatchers.js');
function iterationKey(ts,list) {
  if(!ts.isVariableDeclarationList(list)||!(list.flags&ts.NodeFlags.Const)) return undefined;
  if(list.declarations.length!==1||!ts.isIdentifier(list.declarations[0].name)) return undefined;
  return declarationKey(list.declarations[0]);
}
function callbackCall(bindings,checker,node,key) {
  if(!bindings.ts.isIdentifier(node)||!syntax.keys(bindings,checker,node).includes(key)) return undefined;
  const expression=topExpression(bindings.ts,node),call=expression.parent;
  return bindings.ts.isCallExpression(call)&&call.expression===expression?call:undefined;
}
function invocationSites(bindings,{loop,checker}) {
  const {ts}=bindings,key=iterationKey(ts,loop.initializer),sites=[];
  if(syntax.decoratedOwner(ts,loop)) {bindings.graph.diagnostic('unresolved observer decorated dispatcher');return sites;}
  if(!key) {bindings.graph.diagnostic('unresolved observer mutable iteration binding');return sites;}
  const visit=node=>{
    if(ts.isFunctionLike(node)) return;
    const call=callbackCall(bindings,checker,node,key);
    if(call) sites.push(call);
    ts.forEachChild(node,visit);
  };
  visit(loop.statement);return sites;
}
class Targets {
  constructor(bindings,channel,site) {
    this.bindings=bindings;this.channel=channel;this.site=site;
    this.from=syntax.owner(bindings,site);this.byTarget=new Map();
  }
  evidence(registration,index) {
    const evidence=relationshipEvidence(this.bindings.graph,{rule:'codegraph.observer:collection',semantics:'possible-invocation',site:this.site,
      registration:registration.site,from:this.from,to:index,channel:this.channel.label});
    const id=this.bindings.graph.nodes[registration.from]?.id;
    if(this.supports(id,registration.from,evidence)) evidence.supporting_symbols.push(id);
    return evidence;
  }
  supports(id,index,evidence) {return id&&!evidence.supporting_symbols.includes(id)&&!this.bindings.graph.excluded.has(index);}
  add(registration,index) {
    const evidence=this.evidence(registration,index);
    const result=this.byTarget.get(index)||{index,certainty:'possible',provenance:'codegraph.observer.v1',evidence:[]};
    if(!result.evidence.some(previous=>JSON.stringify(previous)===JSON.stringify(evidence))) result.evidence.push(evidence);
    this.byTarget.set(index,result);
  }
  registration(registration) {for(const index of registration.targets) this.add(registration,index);}
  collect() {
    for(const registration of this.channel.registrations) this.registration(registration);
    return [...this.byTarget.values()];
  }
}
function resolveChannel(bindings,channel) {
  if(!channel.valid) return;
  const sites=channel.dispatches.flatMap(dispatch=>invocationSites(bindings,dispatch));
  for(const site of sites) {
    const targets=new Targets(bindings,channel,site).collect();
    if(targets.length) bindings.invocations.set(site,targets);
  }
}
function collectObservers(bindings) {
  const channels=collectChannels(bindings);
  if(!channels.size) return;
  const uses=collectUses(bindings,channels);
  validateStorage(bindings,channels,uses.writes);
  validateDispatchers(bindings,channels,uses);
  collectRegistrations(bindings,uses);
  for(const channel of channels.values()) resolveChannel(bindings,channel);
}
module.exports={collectObservers};
