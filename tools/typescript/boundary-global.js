'use strict';
const {written}=require('./observer-values.js');
const {globalMemberUse}=require('./intrinsic-globals.js');
const GLOBAL_FETCH='codegraph.boundary:global-fetch-replaced';
function globalFetchEscape(bindings,checker,node) {
  const use=globalMemberUse(bindings,checker,node,'fetch');
  return use===null||!!use&&written(bindings.ts,use);
}
module.exports={GLOBAL_FETCH,globalFetchEscape};
