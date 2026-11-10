export function nextRequest() { return fetch('/api'); }
export function springItems() { return fetch('/api/items'); }
export function springItem() { return fetch('/api/items/42'); }
export function springCreate() { return fetch('/api/items', {method: 'POST'}); }
export function constrainedRequest() { return fetch('/api/constrained'); }
export function externalRequest() { return fetch('https://example.invalid/api/items'); }
export function dynamicRequest(path: string) { return fetch(path); }
export function shadowedRequest() {
  const fetch = (path: string) => path;
  return fetch('/api/items');
}
