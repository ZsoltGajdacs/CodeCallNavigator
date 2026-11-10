'use strict';
const {declarationKey}=require('./paths.js');
const syntax=require('./observer-syntax.js');
const {isWrapper}=require('./syntax.js');
const {storageMembers}=require('./observer-intrinsics.js');
function moduleCollection(ts,node) {
  if(!ts.isVariableDeclaration(node)||!syntax.isConst(ts,node)) return false;
  const statement=node.parent.parent;
  if(!ts.isVariableStatement(statement)||syntax.hasModifier(ts,statement,ts.SyntaxKind.ExportKeyword)) return false;
  return ts.isSourceFile(statement.parent)||ts.isModuleBlock(statement.parent);
}
function privateMember(ts,node) {
  return ts.isPrivateIdentifier(node.name)||syntax.hasModifier(ts,node,ts.SyntaxKind.PrivateKeyword);
}
function privateField(ts,node) {
  if(!ts.isPropertyDeclaration(node)) return false;
  if(!ts.isClassDeclaration(node.parent)||!node.parent.name) return false;
  return syntax.hasModifier(ts,node,ts.SyntaxKind.ReadonlyKeyword)&&privateMember(ts,node);
}
function named(ts,node) {return node.name&&(ts.isIdentifier(node.name)||ts.isPrivateIdentifier(node.name));}
function callableSet(bindings,checker,node) {
  if(!syntax.defaultSet(bindings,checker,node.initializer)) return false;
  const element=checker.getTypeArguments(checker.getTypeAtLocation(node.name))[0];
  return !!element&&checker.getSignaturesOfType(element,bindings.ts.SignatureKind.Call).length>0;
}
function candidate(bindings,node,{checker}) {
  const {ts}=bindings;
  if(!named(ts,node)) return undefined;
  if(!moduleCollection(ts,node)&&!privateField(ts,node)) return undefined;
  if(!callableSet(bindings,checker,node)) return undefined;
  const instance=ts.isPropertyDeclaration(node)&&!syntax.hasModifier(ts,node,ts.SyntaxKind.StaticKeyword);
  return {key:declarationKey(node),node,instance,valid:true,label:syntax.label(ts,node),intrinsics:storageMembers(bindings,checker,node),registrations:[],dispatches:[]};
}
function closedClass(ts,node) {
  if(!ts.isSourceFile(node.parent)||!ts.isExternalModule(node.parent)) return false;
  if(node.heritageClauses?.length) return false;
  return !syntax.hasModifier(ts,node,ts.SyntaxKind.ExportKeyword)&&!syntax.hasModifier(ts,node,ts.SyntaxKind.DefaultKeyword);
}
function classUseAllowed(ts,node,declaration) {
  if(node===declaration.name||syntax.typePosition(ts,node)) return true;
  return ts.isNewExpression(node.parent)&&node.parent.expression===node;
}
function singletonAllocation(ts,node) {
  while(node.parent&&isWrapper(ts,node.parent)) node=node.parent;
  const declaration=node.parent;
  if(!ts.isVariableDeclaration(declaration)||declaration.initializer!==node) return undefined;
  if(!syntax.isConst(ts,declaration)) return undefined;
  const statement=declaration.parent.parent;
  return ts.isVariableStatement(statement)&&ts.isSourceFile(statement.parent)?declaration:undefined;
}
class SingletonClasses {
  constructor(bindings,channels) {
    this.bindings=bindings;this.ts=bindings.ts;this.classes=new Map();
    for(const channel of channels.values()) this.add(channel);
    this.names=new Set([...this.classes.values()].map(info=>info.node.name.text));
  }
  add(channel) {
    if(!channel.instance) return;
    const node=channel.node.parent;
    this.classes.set(declarationKey(node),{node,valid:closedClass(this.ts,node),allocations:0});
  }
  reference(node,checker) {
    if(!this.ts.isIdentifier(node)||!this.names.has(node.text)) return;
    for(const key of syntax.keys(this.bindings,checker,node)) this.checkReference(this.classes.get(key),node);
  }
  checkReference(info,node) {if(info&&!classUseAllowed(this.ts,node,info.node)) info.valid=false;}
  meta(node) {
    if(!this.ts.isMetaProperty(node)||node.keywordToken!==this.ts.SyntaxKind.NewKeyword) return;
    let parent=node.parent;
    while(parent&&!this.ts.isClassDeclaration(parent)) parent=parent.parent;
    const info=parent&&this.classes.get(declarationKey(parent));
    if(info) info.valid=false;
  }
  construction(node,checker) {
    if(!this.ts.isNewExpression(node)) return;
    const info=this.classes.get(syntax.symbolKey(checker.getTypeAtLocation(node.expression).getSymbol()));
    if(!info) return;
    info.allocations++;
    const declaration=singletonAllocation(this.ts,node);
    if(declaration) info.binding=declarationKey(declaration);
    else info.valid=false;
  }
  scan() {
    if(!this.classes.size) return;
    syntax.walkOwned(this.bindings,(node,{checker})=>{
      this.reference(node,checker);this.meta(node);this.construction(node,checker);
    });
  }
  binding(channel) {return this.classes.get(declarationKey(channel.node.parent)).binding;}
  valid(channel) {
    const info=this.classes.get(declarationKey(channel.node.parent));
    return info.valid&&info.allocations===1;
  }
}
function decoratedScope(bindings,channel) {
  if(!syntax.decoratedOwner(bindings.ts,channel.node)) return;
  channel.valid=false;
  bindings.graph.diagnostic('unresolved observer decorated collection ownership');
}
function staticScope(bindings,channel) {
  if(channel.instance) return;
  if(!bindings.ts.isPropertyDeclaration(channel.node)) return;
  const parent=channel.node.parent.parent;
  if([bindings.ts.isSourceFile,bindings.ts.isModuleBlock].some(check=>check(parent))) return;
  channel.valid=false;
  bindings.graph.diagnostic('unresolved observer repeated class scope');
}
function instanceScope(bindings,classes,channel) {
  if(!channel.instance) return;
  if(classes.valid(channel)) {channel.singleton=classes.binding(channel);return;}
  channel.valid=false;
  bindings.graph.diagnostic('unresolved observer instance channels (constructor escape or multiple allocations)');
}
function collectChannels(bindings) {
  const channels=new Map();
  syntax.walkOwned(bindings,(node,entry)=>{
    const channel=candidate(bindings,node,entry);
    if(channel) channels.set(channel.key,channel);
  });
  const classes=new SingletonClasses(bindings,channels);classes.scan();
  for(const channel of channels.values()) {decoratedScope(bindings,channel);staticScope(bindings,channel);}
  for(const channel of channels.values()) instanceScope(bindings,classes,channel);
  return channels;
}
module.exports={collectChannels};
