'use strict';
const {sourceOffsets}=require('./flow-source.js');
function sourceSite(node) {
  const file=node.getSourceFile(),start=node.getStart(file);
  const bytes=sourceOffsets(file),point=file.getLineAndCharacterOfPosition(start);
  return {file:file.fileName,line:point.line+1,column:point.character+1,
    span:{start:bytes[start],end:bytes[node.end]}};
}
function relationshipEvidence(graph,{rule,semantics,site,registration,from,to,channel}) {
  const supporting_symbols=[from,to].map(index=>graph.nodes[index]?.id).filter(Boolean);
  const result={rule,version:1,invocation:sourceSite(site),semantics,supporting_symbols};
  if(registration) result.registration=sourceSite(registration);
  if(channel) result.channel=channel;
  return result;
}
function mergeEvidence(edge,additional) {
  if(!additional.length) return;
  const merged=new Map([...(edge.evidence||[]),...additional].map(value=>[JSON.stringify(value),value]));
  edge.evidence=[...merged.values()];
}
module.exports={sourceSite,relationshipEvidence,mergeEvidence};
