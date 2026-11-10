'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {registrar,directRegistrar}=require('./observer-uses.js');
const {stableTargets}=require('./observer-values.js');
class Registrations {
  constructor(bindings,uses) {this.bindings=bindings;this.uses=uses;this.registrars=new Map();}
  add(channel,{checker,argument,site,from}) {
    const targets=stableTargets(this.bindings,checker,argument,this.uses.writes);
    if(targets.length) channel.registrations.push({targets,site,from});
    else this.bindings.graph.diagnostic('unresolved observer callback registrations');
  }
  store(store) {
    if(!store.channel.valid) return;
    if(syntax.decoratedOwner(this.bindings.ts,store.fn)) {
      store.channel.valid=false;
      this.bindings.graph.diagnostic('unresolved observer decorated registrar');return;
    }
    const registration=registrar(this.bindings,store);
    if(registration&&!this.uses.writes.has(declarationKey(registration.fn))) {
      const existing=this.registrars.get(registration.index)||[];
      existing.push(registration);this.registrars.set(registration.index,existing);
      return;
    }
    this.add(store.channel,{checker:store.checker,argument:store.argument,site:store.call,from:syntax.owner(this.bindings,store.call)});
  }
  call({node,checker}) {
    const targets=this.bindings.targets(checker,node);
    if(targets.length!==1) return;
    for(const registration of this.registrars.get(targets[0])||[]) this.bindCall(registration,node,checker);
  }
  bindCall(registration,node,checker) {
    if(syntax.decoratedOwner(this.bindings.ts,node)) {
      this.bindings.graph.diagnostic('unresolved observer decorated registration');return;
    }
    if(!directRegistrar(this.bindings,{node,checker,writes:this.uses.writes},registration)||node.arguments.some(this.bindings.ts.isSpreadElement)) {
      this.bindings.graph.diagnostic('unresolved observer registrar delegation');return;
    }
    this.add(registration.channel,{checker,argument:node.arguments[registration.parameter],site:node,from:registration.index});
  }
  collect() {
    for(const store of this.uses.stores) this.store(store);
    if(this.registrars.size) for(const call of this.uses.calls) this.call(call);
  }
}
function collectRegistrations(bindings,uses) {new Registrations(bindings,uses).collect();}
module.exports={collectRegistrations};
