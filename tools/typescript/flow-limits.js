'use strict';
/** Bound detached IR independently of compiler graph extraction and JSON nesting limits. */
class FlowLimits {
  constructor() {this.reset();}
  reset() {this.depth=0;this.nodes=0;this.messages=new Set();}
  run(build,fallback) {
    if(this.depth>=24||this.nodes>=1500) {
      const reason='Flow export exceeds its nesting or statement budget; this source region is incomplete.';
      this.messages.add(reason);return fallback(reason);
    }
    this.depth++;this.nodes++;
    try {return build();} finally {this.depth--;}
  }
  exhausted() {return this.nodes>=1500;}
}
module.exports={FlowLimits};
