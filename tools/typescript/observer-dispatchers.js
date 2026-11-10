'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {topExpression}=require('./observer-uses.js');
const {registrarReceiver}=require('./observer-receivers.js');
class Dispatchers {
  constructor(bindings,channels,uses) {this.bindings=bindings;this.channels=channels;this.uses=uses;this.methods=new Map();this.names=new Set();}
  add(channel,dispatch) {
    const fn=syntax.callable(this.bindings.ts,dispatch.loop);
    if(!fn||!this.bindings.ts.isMethodDeclaration(fn)) return;
    const key=declarationKey(fn),existing=this.methods.get(key)||[];
    existing.push({fn,channel});this.methods.set(key,existing);this.names.add(fn.name.getText());
  }
  collect() {
    for(const channel of this.channels.values()) this.channel(channel);
    if(!this.methods.size) return;
    syntax.walkOwned(this.bindings,(node,{checker})=>this.reference(node,checker));
  }
  channel(channel) {
    if(!this.bindings.ts.isPropertyDeclaration(channel.node)) return;
    for(const dispatch of channel.dispatches) this.add(channel,dispatch);
  }
  reference(node,checker) {
    if(!this.bindings.ts.isIdentifier(node)||!this.names.has(node.text)) return;
    for(const key of syntax.keys(this.bindings,checker,node)) this.methods.get(key)?.forEach(method=>this.use(method,node,checker));
  }
  use(method,node,checker) {
    if(node===method.fn.name) return;
    if(this.direct(method,node,checker)) return;
    if(method.channel.valid) this.bindings.graph.diagnostic('unresolved observer dispatcher receiver escape');
    method.channel.valid=false;
  }
  direct(method,node,checker) {
    const {ts}=this.bindings;
    const expression=ts.isPropertyAccessExpression(node.parent)?node.parent:node;
    const reference=topExpression(ts,expression),call=reference.parent;
    if(!ts.isCallExpression(call)||call.expression!==reference) return false;
    return registrarReceiver(this.bindings,{node:call,checker,writes:this.uses.writes},expression,method);
  }
}
function validateDispatchers(bindings,channels,uses) {new Dispatchers(bindings,channels,uses).collect();}
module.exports={validateDispatchers};
