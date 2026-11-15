'use strict';
const {FlowSource}=require('./flow-source.js');
const {FlowStatements}=require('./flow-statements.js');
const {diagnostics}=require('./flow-diagnostics.js');
const {execution}=require('./flow-execution.js');
function flowInventory(bindings) {
  const functions=[];
  for(const file of bindings.files) {
    const source=new FlowSource(bindings,file),statements=new FlowStatements(source);
    for(const [index,declaration] of bindings.byIndex) {
      if(declaration.getSourceFile()!==file.file) continue;
      if(!declaration.body) continue;
      if(bindings.graph.excluded.has(index)) continue;
      const node=bindings.graph.nodes[index];
      source.limits.reset();
      const body=statements.functionBody(declaration);
      const messages=[...diagnostics(bindings.ts,declaration),...source.limits.messages];
      const functionFlow={symbol:node.id,file:node.file,build:node.build,project:node.project,language:'typescript',span:source.span(declaration),
        body,diagnostics:messages};
      const timing=execution(bindings.ts,declaration);
      if(timing) functionFlow.execution=timing;
      functions.push(functionFlow);
    }
  }
  return {version:1,functions};
}
module.exports={flowInventory};
