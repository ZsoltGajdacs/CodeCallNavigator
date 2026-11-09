'use strict';
const fs=require('node:fs');
const path=require('node:path');
const {createRequire}=require('node:module');
const {canonical}=require('./paths.js');
const MANIFESTS = ['package.json','package-lock.json','npm-shrinkwrap.json','pnpm-lock.yaml','yarn.lock','.pnp.cjs'];
function projectInputs(config, inputs) {
  let directory = path.dirname(config);
  while (true) {
    for (const name of MANIFESTS) inputs.add(path.join(directory,name));
    const parent = path.dirname(directory);
    if (parent === directory) break;
    directory = parent;
  }
}
function packageVersion(config, name, inputs, versions) {
  const resolver = createRequire(path.join(path.dirname(config),'__codegraph__.js'));
  try {
    const manifest = canonical(resolver.resolve(`${name}/package.json`));
    inputs.add(manifest);
    versions[`${name}:${config}`] = JSON.parse(fs.readFileSync(manifest,'utf8')).version;
  } catch (error) {
    if (['MODULE_NOT_FOUND','ERR_PACKAGE_PATH_NOT_EXPORTED'].includes(error.code)) return;
    throw error;
  }
}
module.exports={projectInputs,packageVersion};
