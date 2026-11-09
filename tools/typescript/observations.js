'use strict';
const {createHash}=require('node:crypto');
function digest(value) {
  return createHash('sha256').update(JSON.stringify(value) ?? 'undefined').digest('hex');
}
class Observations {
  constructor() { this.values=new Map(); }
  observe(key, read) {
    const value=read(), hash=digest(value), previous=this.values.get(key);
    if (previous && previous.hash!==hash) throw new Error(`TypeScript input changed during preparation: ${key}`);
    if (!previous) this.values.set(key,{hash,read});
    return value;
  }
  verify() {
    for (const [key,{hash,read}] of this.values) {
      if (digest(read())!==hash) throw new Error(`TypeScript input changed after preparation: ${key}`);
    }
  }
}
module.exports={Observations};
