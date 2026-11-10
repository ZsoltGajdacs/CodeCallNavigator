const choose = Date.now() > 0;
function nullishPrimary() { return 1; }
function nullishFallback() { return 2; }
function orPrimary() { return 3; }
function orFallback() { return 4; }
function aliasPrimary() { return 5; }
function aliasFallback() { return 6; }
function parameterLeaf() { return 7; }
function returnLeafA() { return 8; }
function returnLeafB() { return 9; }
function mutationLeafA() { return 10; }
function mutationLeafB() { return 11; }
function forwardedLeaf() { return 12; }

export function logicalNullish() {
  const maybe = choose ? nullishPrimary : undefined;
  const selected = maybe ?? nullishFallback;
  return selected();
}
export function logicalOr() {
  const maybe = choose ? orPrimary : undefined;
  const selected = maybe || orFallback;
  return selected();
}
export function immutableAlias() {
  const maybe = choose ? aliasPrimary : undefined;
  const selected = maybe ?? aliasFallback;
  const alias = selected;
  return alias();
}
function invokeParameter(callback: () => number) { return callback(); }
export function callParameter() { return invokeParameter(parameterLeaf); }
function identity(callback: () => number) { return callback; }
export function returnedA() { const callback = identity(returnLeafA); return callback(); }
export function returnedB() { const callback = identity(returnLeafB); return callback(); }
export function immediateReturned() { return identity(returnLeafA)(); }
function relay(callback: () => number) { return callback; }
function forwardOnly(callback: () => number) { return relay(callback); }
export function forwardingOnly() { forwardOnly(forwardedLeaf); return 0; }
function mutatedParameter(callback: () => number) { callback = mutationLeafB; return callback(); }
export function parameterMutationNegative() { return mutatedParameter(mutationLeafA); }
export function mutableAliasNegative() {
  let callback = choose ? mutationLeafA : mutationLeafB;
  callback = mutationLeafB;
  const selected = callback ?? mutationLeafA;
  return selected();
}
export function mutableReturnNegative() {
  let callback = identity(mutationLeafA);
  callback = identity(mutationLeafB);
  const alias = callback;
  return alias();
}
