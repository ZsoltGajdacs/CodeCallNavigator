'use strict';
const {declarationKey}=require('./paths.js');
// Falsy means non-nullish falsy values; ?? preserves false/0/"", while || skips them.
function value(flags={}) {return {targets:new Map(),nullish:false,falsy:false,truthy:false,unsafe:false,...flags};}
function unknown(reason) {return value({nullish:true,falsy:true,truthy:true,unsafe:true,reason});}
function target(index) {return {...value({truthy:true}),targets:new Map([[index,new Map()]])};}
function merge(values) {
  const result=value();
  for(const item of values) {
    mergeFlags(result,item);
    mergeTargets(result.targets,item.targets);
  }
  return result;
}
function mergeFlags(result,item) {
  for(const flag of ['nullish','falsy','truthy','unsafe']) result[flag] ||= item[flag];
  result.reason ||= item.reason;
}
function mergeTargets(result,targets) {
  for(const [index,origins] of targets) {
    const previous=result.get(index)||new Map();
    for(const [key,origin] of origins) previous.set(key,origin);
    result.set(index,previous);
  }
}
function derived(input,origin) {
  const result=merge([input]);
  const key=`${origin.kind}:${declarationKey(origin.site)}:${origin.support??''}`;
  for(const origins of result.targets.values()) origins.set(key,origin);
  return result;
}
function fingerprint(input) {
  const targets=[...input.targets].map(([index,origins])=>[index,[...origins.keys()].sort()]).sort((a,b)=>a[0]-b[0]);
  return JSON.stringify([input.nullish,input.falsy,input.truthy,input.unsafe,targets]);
}
module.exports={value,unknown,target,merge,derived,fingerprint};
