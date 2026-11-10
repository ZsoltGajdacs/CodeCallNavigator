'use strict';
const fs = require('node:fs');
const path = require('node:path');
function canonical(file) {
  const absolute = path.resolve(file);
  return fs.existsSync(absolute) ? fs.realpathSync(absolute) : absolute;
}
function isProjectSource(file) {
  if (!/\.(?:tsx?|mts|cts)$/.test(file)) return false;
  if (/\.d\.(?:ts|mts|cts)$/.test(file)) return false;
  return !file.includes('/node_modules/');
}
// Compiler AST snapshots are immutable. Cache node identities without caching
// arbitrary filesystem probes used by preparation and freshness verification.
const declarationKeys=new WeakMap();
const sourcePaths=new WeakMap();
function declarationKey(node) {
  if(!declarationKeys.has(node)) {
    const file=node.getSourceFile();
    if(!sourcePaths.has(file)) sourcePaths.set(file,canonical(file.fileName));
    declarationKeys.set(node,`${sourcePaths.get(file)}:${node.getStart()}`);
  }
  return declarationKeys.get(node);
}
module.exports = {canonical, isProjectSource, declarationKey};
