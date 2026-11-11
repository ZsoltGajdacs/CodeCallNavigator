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
function resolveManifest(config,name) {
  const resolver=createRequire(path.join(path.dirname(config),'__codecallnavigator__.js'));
  try { return canonical(resolver.resolve(`${name}/package.json`)); }
  catch(error) {
    if (['MODULE_NOT_FOUND','ERR_PACKAGE_PATH_NOT_EXPORTED'].includes(error.code)) return null;
    throw error;
  }
}
function packageVersion(config, name, inputs, {versions,observations}) {
  const manifest=observations.observe(`package:${config}:${name}`,()=>resolveManifest(config,name));
  if (!manifest) return;
  inputs.add(manifest);
  const contents=observations.observe(`manifest:${manifest}`,()=>fs.readFileSync(manifest,'utf8'));
  versions[`${name}:${config}`]=JSON.parse(contents).version;
}
module.exports={projectInputs,packageVersion};
