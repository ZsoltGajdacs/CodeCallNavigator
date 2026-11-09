'use strict';
const {declarationKey}=require('./paths.js');
const {isJsxSite}=require('./syntax.js');
function jsxProperty(ts,property) {
  if(!ts.isJsxAttribute(property)) return undefined;
  if(!property.initializer) return undefined;
  if(!ts.isJsxExpression(property.initializer)) return undefined;
  return {name:property.name.getText(),expression:property.initializer.expression};
}
function objectProperty(ts,property) {
  if(ts.isPropertyAssignment(property)) return {name:property.name.getText(),expression:property.initializer};
  if(ts.isShorthandPropertyAssignment(property)) return {name:property.name.getText(),expression:property.name};
  return undefined;
}
function bindingKeys(ts,parameter,name) {
  if(!ts.isObjectBindingPattern(parameter.name)) return [`${declarationKey(parameter)}:${name}`];
  return parameter.name.elements.filter(element=>(element.propertyName||element.name).getText()===name).map(declarationKey);
}
function mergeBinding(props,key,targets) {
  const previous=props.get(key)||new Set();
  const size=previous.size;
  for(const target of targets) previous.add(target);
  props.set(key,previous);
  return size!==previous.size;
}
function bindParameter(bindings,parameter,property,targets) {
  const keys=bindingKeys(bindings.ts,parameter,property);
  return keys.map(key=>mergeBinding(bindings.props,key,targets)).some(Boolean);
}
function bindProperty(bindings,site,property,components) {
  const values=bindings.resolve(site.checker,property.expression);
  let changed=false;
  for(const component of components) {
    const parameter=bindings.byIndex.get(component)?.parameters?.[0];
    if(parameter) changed=bindParameter(bindings,parameter,property.name,values)||changed;
  }
  return changed;
}
function elementSite(ts,checker,node,reactImport) {
  if(reactImport(ts,checker,node.expression)!=='createElement') return undefined;
  const props=node.arguments[1];
  if(!props) return undefined;
  if(!ts.isObjectLiteralExpression(props)) return undefined;
  return {checker,tag:node.arguments[0],properties:props.properties.map(property=>objectProperty(ts,property)).filter(Boolean)};
}
function propSite(ts,checker,node,reactImport) {
  if(isJsxSite(ts,node)) return {checker,tag:node.tagName,properties:node.attributes.properties.map(property=>jsxProperty(ts,property)).filter(Boolean)};
  if(ts.isCallExpression(node)) return elementSite(ts,checker,node,reactImport);
  return undefined;
}
function collectSites(bindings,reactImport) {
  const sites=[];
  for(const {file,checker} of bindings.files) {
    const visit=node=>{
      const site=propSite(bindings.ts,checker,node,reactImport);
      if(site) sites.push(site);
      bindings.ts.forEachChild(node,visit);
    };
    visit(file);
  }
  return sites;
}
function bindSite(bindings,site) {
  const components=bindings.resolve(site.checker,site.tag);
  return site.properties.map(property=>bindProperty(bindings,site,property,components)).some(Boolean);
}
function collectProps(bindings,reactImport) {
  const sites=collectSites(bindings,reactImport);
  // The finite target sets only grow, so cycles converge without an arbitrary depth limit.
  let changed=true;
  while(changed) changed=sites.map(site=>bindSite(bindings,site)).some(Boolean);
}
module.exports={collectProps,jsxProperty,objectProperty};
