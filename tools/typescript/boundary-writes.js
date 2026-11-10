'use strict';
const {collectFacts}=require('./callable-facts.js');
const {declarationKey}=require('./paths.js');
const {written}=require('./observer-values.js');
const {walkOwned}=require('./observer-syntax.js');
const {GLOBAL_FETCH,globalFetchEscape}=require('./boundary-global.js');
function boundaryWrites(bindings) {
  const {writes}=collectFacts(bindings),{ts}=bindings;
  walkOwned(bindings,(node,{checker})=>{
    if(globalFetchEscape(bindings,checker,node)) writes.add(GLOBAL_FETCH);
    if(!ts.isPropertyAccessExpression(node)&&!ts.isElementAccessExpression(node)) return;
    if(!written(ts,node)) return;
    const symbol=bindings.symbol(checker,node);
    for(const declaration of symbol?.declarations||[]) writes.add(declarationKey(declaration));
  });
  return writes;
}
module.exports={boundaryWrites};
