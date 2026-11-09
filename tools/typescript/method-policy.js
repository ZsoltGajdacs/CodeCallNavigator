'use strict';
// Compiler discovery supplies facts; this policy owns the graph inclusion decision.
function excludedMethod({kind,objectMember}) {
  return objectMember||['constructor','implicit-constructor','initializer'].includes(kind);
}
module.exports={excludedMethod};
