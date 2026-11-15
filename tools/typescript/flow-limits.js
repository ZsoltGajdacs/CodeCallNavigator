'use strict';
const MAX_FLOW_NODES=1500;
/** Bound detached IR independently of compiler graph extraction and JSON nesting limits. */
class FlowLimits {
  constructor() {this.reset();}
  reset() {this.depth=0;this.nodes=0;this.messages=new Set();}
  run(build,fallback) {
    if(this.depth>=24||this.nodes>=MAX_FLOW_NODES) {
      const reason='Flow export exceeds its nesting or statement budget; this source region is incomplete.';
      this.messages.add(reason);return fallback(reason);
    }
    this.depth++;this.nodes++;
    try {return build();} finally {this.depth--;}
  }
  exhausted() {return this.nodes>=MAX_FLOW_NODES;}
}
module.exports={FlowLimits,MAX_FLOW_NODES};
