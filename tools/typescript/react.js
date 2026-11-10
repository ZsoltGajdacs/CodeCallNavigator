'use strict';
const {collectProps,jsxProperty,objectProperty}=require('./props.js');
const {relationshipEvidence}=require('./evidence.js');
const CALLBACK_APIS=new Set(['useEffect','useLayoutEffect','useInsertionEffect','useMemo','memo','forwardRef']);
const HOOK_KINDS={useEffect:'react-effect',useLayoutEffect:'react-effect',useInsertionEffect:'react-effect',useMemo:'react-memo',useState:'react-initializer',useReducer:'react-reducer'};
class ReactRules {
  constructor(bindings,graph,reactImport) {
    this.bindings=bindings;this.graph=graph;this.ts=bindings.ts;this.reactImport=reactImport;
  }
  add(from,targets,kind,site) {
    const semantics=this.semantics(kind);
    for(const to of targets) {
      const evidence=relationshipEvidence(this.graph,{rule:`codegraph.react:${kind}`,semantics,site,registration:site,from,to});
      this.graph.edge(from,to,kind,{certainty:'possible',provenance:`codegraph.react.v1:${kind}`,evidence:[evidence]});
    }
  }
  semantics(kind) {
    if(['react-event','react-prop'].includes(kind)) return 'reference';
    if(['react-effect','react-cleanup','react-reducer'].includes(kind)) return 'deferred';
    return 'possible-invocation';
  }
  resolve(checker,expression) { return this.bindings.resolve(checker,expression); }
  hook(checker,node,from,name) {
    const kind=HOOK_KINDS[name];
    if(!kind) return;
    const callbacks=this.resolve(checker,node.arguments[0]);
    this.add(from,callbacks,kind,node);
    if(name==='useReducer') this.add(from,this.resolve(checker,node.arguments[2]),'react-initializer',node);
    if(kind==='react-effect') for(const index of callbacks) this.cleanup(checker,index);
  }
  cleanup(checker,index) {
    const declaration=this.bindings.byIndex.get(index);
    if(!declaration?.body) return;
    if(!this.ts.isBlock(declaration.body)) {
      this.add(index,this.resolve(checker,declaration.body),'react-cleanup',declaration.body);
      return;
    }
    const visit=node=>{
      if(this.ts.isReturnStatement(node)) this.add(index,this.resolve(checker,node.expression),'react-cleanup',node);
      if(node!==declaration.body && this.ts.isFunctionLike(node)) return;
      this.ts.forEachChild(node,visit);
    };
    visit(declaration.body);
  }
  missingCallback(checker,node,name) {
    if(!CALLBACK_APIS.has(name)) return false;
    if(!node.arguments.length) return false;
    return !this.resolve(checker,node.arguments[0]).length;
  }
  call(checker,node,from) {
    const name=this.reactImport(this.ts,checker,node.expression);
    if(name==='createElement') {
      this.add(from,this.resolve(checker,node.arguments[0]),'react-render',node);
      this.objectProps(checker,node.arguments[1],from);
    } else if(name) this.hook(checker,node,from,name);
    if(this.missingCallback(checker,node,name)) this.graph.diagnostic('unresolved React callback bindings');
  }
  objectProps(checker,object,from) {
    if(!object) return;
    if(!this.ts.isObjectLiteralExpression(object)) return;
    for(const property of object.properties) {
      const binding=objectProperty(this.ts,property);
      if(binding) this.add(from,this.resolve(checker,binding.expression),/^on[A-Z]/.test(binding.name)?'react-event':'react-prop',property);
      else this.graph.diagnostic('unresolved React spread/delegation bindings');
    }
  }
  component(checker,tag,from) {
    if(this.ts.isIdentifier(tag) && /^[a-z]/.test(tag.text)) return;
    const targets=this.resolve(checker,tag);
    this.add(from,targets,'react-render',tag);
    if(!targets.length) this.graph.diagnostic('unresolved React component bindings');
  }
  attribute(checker,property,from) {
    if(this.ts.isJsxSpreadAttribute(property)) {
      this.graph.diagnostic('unresolved React spread/delegation bindings');
      return;
    }
    const binding=jsxProperty(this.ts,property);
    if(!binding?.expression) return;
    const targets=this.resolve(checker,binding.expression);
    const event=/^on[A-Z]/.test(binding.name);
    this.add(from,targets,event?'react-event':'react-prop',property);
    if(event && !targets.length) this.graph.diagnostic('unresolved React event/delegation bindings');
  }
  jsx(checker,node,from) {
    this.component(checker,node.tagName,from);
    for(const property of node.attributes.properties) this.attribute(checker,property,from);
  }
  collectProps() { collectProps(this.bindings,this.reactImport); }
}
module.exports={ReactRules};
