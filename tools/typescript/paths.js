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
function declarationKey(node) {
  return `${canonical(node.getSourceFile().fileName)}:${node.getStart()}`;
}
module.exports = {canonical, isProjectSource, declarationKey};
