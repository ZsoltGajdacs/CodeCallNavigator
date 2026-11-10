'use strict';
const {literal}=require('./boundary-api.js');
const OPTIONS=new Set(['method','headers','body','cache','credentials','mode','redirect','referrer','referrerPolicy','integrity','keepalive','signal','next']);
const METHODS=new Set(['GET','POST','PUT','PATCH','DELETE','HEAD','OPTIONS']);
function optionName(ts,property) {
  if(!ts.isPropertyAssignment(property)) return undefined;
  if(ts.isComputedPropertyName(property.name)) return undefined;
  return OPTIONS.has(property.name.text)?property.name.text:undefined;
}
function optionsMethod(ts,options) {
  let method='GET';const seen=new Set();
  for(const property of options.properties) {
    const name=optionName(ts,property);
    if(!name||seen.has(name)) return undefined;
    seen.add(name);
    if(name==='method') method=literal(ts,property.initializer)?.toUpperCase();
  }
  return METHODS.has(method)?method:undefined;
}
function fetchMethod(ts,node) {
  if(node.arguments.length===1) return 'GET';
  if(node.arguments.length!==2) return undefined;
  const options=node.arguments[1];
  return ts.isObjectLiteralExpression(options)?optionsMethod(ts,options):undefined;
}
module.exports={fetchMethod};
