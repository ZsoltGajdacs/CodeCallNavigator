'use strict';
const syntax=require('./observer-syntax.js');
const {declarationKey}=require('./paths.js');
const {isWrapper}=require('./syntax.js');
const {parameterUnchanged,written}=require('./observer-values.js');
const {elementWrites,setEscape,SET_PROOF}=require('./observer-intrinsics.js');
const {registrarReceiver,collectionReceiver}=require('./observer-receivers.js');
const {ContextEscapes}=require('./observer-context.js');
function wrapperParent(ts,node) {
  if(!node.parent||!isWrapper(ts,node.parent)) return false;
  return node.parent.expression===node;
}
function topExpression(ts,node) {
  while(wrapperParent(ts,node)) node=node.parent;
  return node;
}
function parameter(bindings,store) {
  const {ts}=bindings,{fn,argument,checker}=store;
  if(!fn?.body||!ts.isIdentifier(argument)) return undefined;
  const keys=syntax.keys(bindings,checker,argument);
  return fn.parameters.find(parameter=>ts.isIdentifier(parameter.name)&&keys.includes(declarationKey(parameter)));
}
function plainParameter(parameter) {
  if(!parameter) return false;
  return !parameter.dotDotDotToken&&!parameter.initializer;
}
function registrar(bindings,store) {
  const found=parameter(bindings,store);
  if(!plainParameter(found)) return undefined;
  if(!parameterUnchanged(bindings,store.checker,found,store.fn.body)) return undefined;
  return {index:bindings.index(store.fn),fn:store.fn,parameter:store.fn.parameters.indexOf(found),channel:store.channel};
}
function registrarDeclaration(bindings,declaration,registration) {
  const {ts}=bindings;
  if(declarationKey(declaration)===declarationKey(registration.fn)) return true;
  if(!ts.isVariableDeclaration(declaration)||!syntax.isConst(ts,declaration)) return false;
  const value=syntax.unwrap(ts,declaration.initializer);
  return !!value&&declarationKey(value)===declarationKey(registration.fn);
}
function directRegistrar(bindings,site,registration) {
  const expression=syntax.unwrap(bindings.ts,site.node.expression);
  if(!expression||bindings.ts.isElementAccessExpression(expression)) return false;
  if(!registrarReceiver(bindings,site,expression,registration)) return false;
  const declarations=bindings.symbol(site.checker,expression)?.declarations||[];
  return declarations.some(declaration=>registrarDeclaration(bindings,declaration,registration));
}
class Uses {
  constructor(bindings,channels) {
    this.bindings=bindings;this.channels=channels;this.ts=bindings.ts;
    this.stores=[];this.calls=[];this.writes=new Set();
    this.names=new Set([...channels.values()].map(channel=>channel.node.name.text));
    this.contextual=new ContextEscapes(bindings,this);
  }
  invalidate(channel) {
    if(channel.valid) this.bindings.graph.diagnostic('unresolved observer collection mutation, escape or unsupported use');
    channel.valid=false;
  }
  validStore(call) {
    if(call.arguments.length!==1||this.ts.isSpreadElement(call.arguments[0])) return false;
    return this.ts.isExpressionStatement(call.parent);
  }
  store(channel,call,checker) {
    if(!this.validStore(call)) {this.invalidate(channel);return;}
    this.stores.push({channel,call,checker,argument:call.arguments[0],fn:syntax.callable(this.ts,call)});
  }
  addCall(node,checker) {
    const parent=node.parent;
    if(!this.ts.isPropertyAccessExpression(parent)||parent.expression!==node) return undefined;
    if(!syntax.builtin(this.bindings,checker,parent.name,'add')) return undefined;
    const call=parent.parent;
    return this.ts.isCallExpression(call)&&call.expression===parent?call:undefined;
  }
  iteration(node) {return this.ts.isForOfStatement(node.parent)&&node.parent.expression===node&&!node.parent.awaitModifier;}
  use(channel,expression,checker) {
    if(!collectionReceiver(this.bindings,{checker,writes:this.writes},expression,channel)) {this.invalidate(channel);return;}
    const node=topExpression(this.ts,expression);
    if(this.iteration(node)) {channel.dispatches.push({loop:node.parent,checker});return;}
    const call=this.addCall(node,checker);
    if(call) this.store(channel,call,checker);
    else this.invalidate(channel);
  }
  classChannels(expression,checker,includeStatic=false) {return this.typedChannels(checker.getTypeAtLocation(syntax.unwrap(this.ts,expression)),includeStatic);}
  typedChannels(type,includeStatic=false) {
    const key=syntax.symbolKey(type.getSymbol());
    return [...this.channels.values()].filter(channel=>(channel.instance||includeStatic)&&this.ts.isPropertyDeclaration(channel.node)&&declarationKey(channel.node.parent)===key);
  }
  computed(node,checker) {
    if(written(this.ts,node)) for(const key of elementWrites(this.bindings,node,checker)) this.writes.add(key);
    const direct=syntax.keys(this.bindings,checker,node).map(key=>this.channels.get(key)).filter(Boolean);
    for(const channel of [...direct,...this.classChannels(node.expression,checker,true)]) this.invalidate(channel);
  }
  constructorAccess(node,checker) {
    if(!this.ts.isPropertyAccessExpression(node)||node.name.text!=='constructor') return;
    for(const channel of this.classChannels(node.expression,checker)) this.invalidate(channel);
  }
  assertion(node,checker) {
    if(!this.ts.isAsExpression(node)&&!this.ts.isTypeAssertionExpression(node)) return;
    const source=checker.getTypeAtLocation(node.expression),target=checker.getTypeAtLocation(node);
    if(syntax.symbolKey(source.getSymbol())===syntax.symbolKey(target.getSymbol())) return;
    for(const channel of [...this.typedChannels(source,true),...this.typedChannels(target,true)]) this.invalidate(channel);
  }
  untypedVariable(node,checker) {
    if(!this.ts.isVariableDeclaration(node)||!node.initializer) return;
    const type=checker.getTypeAtLocation(node.name);
    if(!(type.flags&(this.ts.TypeFlags.Any|this.ts.TypeFlags.Unknown))) return;
    for(const channel of this.classChannels(node.initializer,checker,true)) this.invalidate(channel);
  }
  reference(node,checker) {
    if(!this.ts.isIdentifier(node)&&!this.ts.isPrivateIdentifier(node)) return;
    const expression=this.ts.isPropertyAccessExpression(node.parent)&&node.parent.name===node?node.parent:node;
    this.recordReference(node,expression,checker);
  }
  recordReference(node,expression,checker) {
    const writing=written(this.ts,expression);
    if(!this.names.has(node.text)&&!writing) return;
    const keys=syntax.keys(this.bindings,checker,node);
    if(writing) for(const key of keys) this.writes.add(key);
    for(const key of keys) this.channelReference(this.channels.get(key),node,expression,checker);
  }
  channelReference(channel,node,expression,checker) {if(channel&&node!==channel.node.name) this.use(channel,expression,checker);}
  scan() {
    syntax.walkOwned(this.bindings,(node,{checker})=>{
      if(setEscape(this.bindings,checker,node)) this.writes.add(SET_PROOF);
      if(this.ts.isCallExpression(node)) this.calls.push({node,checker});
      if(this.ts.isElementAccessExpression(node)) this.computed(node,checker);
      this.contextual.inspect(node,checker);
      this.constructorAccess(node,checker);this.assertion(node,checker);this.untypedVariable(node,checker);this.reference(node,checker);
    });
    return this;
  }
}
function collectUses(bindings,channels) {return new Uses(bindings,channels).scan();}
module.exports={collectUses,registrar,directRegistrar,topExpression};
