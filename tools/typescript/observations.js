'use strict';
const {createHash}=require('node:crypto');
function digest(value) {
  return createHash('sha256').update(JSON.stringify(value) ?? 'undefined').digest('hex');
}
class Observations {
  constructor() { this.values=new Map(); }
  observe(key, read, directoryListing=false) {
    const value=read(), hash=digest(value), previous=this.values.get(key);
    if (previous && previous.hash!==hash) throw new Error(`TypeScript input changed during preparation: ${key}`);
    if (!previous) this.values.set(key,{hash,read,directoryListing});
    return value;
  }
  verify(recheckMembership) {
    const listings=[];
    for (const [key,observation] of this.values) {
      const hash=digest(observation.read());
      if (hash===observation.hash) continue;
      if (!observation.directoryListing || !recheckMembership) {
        throw new Error(`TypeScript input changed after preparation: ${key}`);
      }
      listings.push({key,observation,hash});
    }
    if (listings.length) {
      recheckMembership(listings[0].key);
      for (const {observation,hash} of listings) observation.hash=hash;
    }
  }
}
module.exports={Observations};
