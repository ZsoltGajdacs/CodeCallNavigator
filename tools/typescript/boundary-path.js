'use strict';
const {literal}=require('./boundary-api.js');
function unsafeSegment(segment) {return !segment||segment==='.'||segment==='..';}
function safeOrigin(value) {
  if(typeof value!=='string') return false;
  if(!value.startsWith('/')) return false;
  if(value.startsWith('//')) return false;
  return !/[\\%\x00-\x20]/.test(value);
}
function pathLiteral(ts,node) {
  const value=literal(ts,node);
  if(!safeOrigin(value)) return undefined;
  const pathname=value.split(/[?#]/,1)[0];
  if(pathname==='/') return value;
  return pathname.slice(1).split('/').some(unsafeSegment)?undefined:value;
}
module.exports={pathLiteral};
