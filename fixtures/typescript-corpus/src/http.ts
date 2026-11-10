export function nextRequest() { return fetch('/api'); }
export function springItems() { return fetch('/api/items'); }
export function springItem() { return fetch('/api/items/42'); }
export function springCreate() { return fetch('/api/items', {method: 'POST'}); }
export function constrainedRequest() { return fetch('/api/constrained'); }
export function springHealth() { return fetch('/api/health'); }
export function aspReady() { return fetch('/ready'); }
export function aspCaseCollision() { return fetch('/api/items/CASEONLY'); }
export function aspOrderCollision() { return fetch('/api/items/ordered'); }
export function aspConstraintCollision() { return fetch('/api/items/constraint/42'); }
export function externalRequest() { return fetch('https://example.invalid/api/items'); }
export function dynamicRequest(path: string) { return fetch(path); }
export function shadowedRequest() {
  const fetch = (path: string) => path;
  return fetch('/api/items');
}
