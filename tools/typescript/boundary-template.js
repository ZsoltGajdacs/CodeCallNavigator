'use strict';
const fs=require('node:fs');
const path=require('node:path');
function appFolders(owner,file) {
  const root=path.dirname(owner);
  const relative=path.relative(root,file.fileName).replaceAll(path.sep,'/');
  const match=/^(src\/)?app\/(.*\/)?(?:page|route)\.tsx?$/.exec(relative);
  if(!match) return undefined;
  if(match[1]&&fs.existsSync(path.join(root,'app'))) return undefined;
  const folders=match[2]?match[2].slice(0,-1).split('/'):[];
  return folders.some(segment=>segment.startsWith('_'))?undefined:folders;
}
function routePath(folders) {
  const segments=[];
  for(const segment of folders) {
    if(/^\([^)]*\)$/.test(segment)) continue;
    if(/^\[[A-Za-z_$][\w$]*\]$/.test(segment)) {segments.push(`{${segment.slice(1,-1)}}`);continue;}
    if(/[\[\]@()]/.test(segment)) return {path:null,restrictions:['unsupported-route-segment']};
    segments.push(segment);
  }
  return {path:`/${segments.join('/')}`,restrictions:[]};
}
function configurationFiles(owner) {
  const root=path.dirname(owner);
  const names=['next.config.js','next.config.mjs','next.config.ts','middleware.ts','middleware.js','proxy.ts','proxy.js','src/middleware.ts','src/middleware.js','src/proxy.ts','src/proxy.js'];
  return names.map(name=>path.join(root,name));
}
function configurationRestrictions(owner) {
  return configurationFiles(owner).some(file=>fs.existsSync(file))?['unmodeled-next-configuration']:[];
}
module.exports={appFolders,routePath,configurationRestrictions,configurationFiles};
