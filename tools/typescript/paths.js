'use strict';
const fs = require('node:fs');
const path = require('node:path');
function canonical(file) {
  let ancestor=path.resolve(file);
  const suffix=[];
  // Missing compiler probes still belong to the physical directory reached
  // through a symlink. Resolve its nearest existing ancestor, not the leaf.
  while (true) {
    try { return path.join(fs.realpathSync(ancestor),...suffix.reverse()); }
    catch(error) {
      if (!['ENOENT','ENOTDIR'].includes(error.code)) throw error;
      const parent=path.dirname(ancestor);
      if (parent===ancestor) throw error;
      suffix.push(path.basename(ancestor));
      ancestor=parent;
    }
  }
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
function sourcePath(file) {
  if(!sourcePaths.has(file)) sourcePaths.set(file,canonical(file.fileName));
  return sourcePaths.get(file);
}
function declarationKey(node) {
  if(!declarationKeys.has(node)) {
    const file=node.getSourceFile();
    declarationKeys.set(node,`${sourcePath(file)}:${node.getStart()}`);
  }
  return declarationKeys.get(node);
}
module.exports = {canonical, isProjectSource, declarationKey, sourcePath};
