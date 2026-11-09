'use strict';
const path = require('node:path');
class Graph {
  constructor(context) {
    this.context=context;
    this.nodes=[]; this.edges=[]; this.indices=new Map(); this.edgeIndices=new Map();
    this.excluded=new Set(); this.framework=new Set(); this.unresolved=new Map();
  }
  node(declaration, owner, kind, {label,checker}) {
    const file=declaration.getSourceFile();
    const position=kind==='module' ? 0 : declaration.getStart(file);
    const id=`${owner}|${file.fileName}|${position}|${kind}`;
    if (this.indices.has(id)) return this.indices.get(id);
    const index=this.nodes.length;
    const relative=path.relative(path.dirname(owner),file.fileName);
    const signature=['module','implicit-constructor','initializer'].includes(kind) ? undefined : checker.getSignatureFromDeclaration(declaration);
    const generatedFolders=['/.next/','/generated/','/__generated__/'];
    const generatedOutput=this.context.metadata.output_roots.some(root=>file.fileName.startsWith(`${root}/`));
    this.nodes.push({id,project:path.basename(path.dirname(owner)),build:owner,language:'typescript',kind,label,
      signature:signature ? checker.signatureToString(signature) : label,namespace:path.dirname(relative),container:relative,file:file.fileName,
      line:file.getLineAndCharacterOfPosition(position).line+1,is_test:/(?:^|\/)(?:__tests__|tests?|test-support|test-fixtures)(?:\/|$)|\.(?:test|spec)\.[cm]?tsx?$/.test(relative),
      is_generated:generatedOutput || generatedFolders.some(root=>file.fileName.includes(root)),body_unavailable:!['module','implicit-constructor','initializer'].includes(kind) && !declaration.body});
    this.indices.set(id,index); return index;
  }
  edge(from,to,kind='call',{certainty='direct',provenance='typescript.compiler'}={}) {
    if (from===undefined || to===undefined) return;
    const key=JSON.stringify([from,to,kind,certainty,provenance]);
    const previous=this.edgeIndices.get(key);
    if (previous!==undefined) { this.edges[previous].count++; return; }
    this.edgeIndices.set(key,this.edges.length);
    this.edges.push({from,to,kind,certainty,count:1,provenance});
  }
  diagnostic(kind) { this.unresolved.set(kind,(this.unresolved.get(kind)||0)+1); }
  result() {
    const called=new Set(this.edges.filter(e=>e.kind!=='reference').map(e=>e.to));
    const candidates=this.nodes.filter((n,i)=>!called.has(i)&&!n.body_unavailable).map(n=>n.id).sort();
    return {schema:'codegraph.typescript.v2',excluded_methods:[...this.excluded].map(i=>this.nodes[i].id),fingerprints:[],engine_versions:'',nodes:this.nodes,edges:this.edges,
      diagnostics:[...this.unresolved].sort().map(([kind,count])=>`${count} ${kind}`),roots:this.context.metadata.configs,
      entry_points:{candidates,application_starts:[],framework_entries:[...this.framework].map(i=>this.nodes[i].id).sort()},versions:this.context.metadata.versions};
  }
}
module.exports={Graph};
