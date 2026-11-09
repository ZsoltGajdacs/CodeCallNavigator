'use strict';
const {isCallback}=require('./syntax.js');
function callableKind(ts,node) {
  if(ts.isConstructorDeclaration(node)) return 'constructor';
  return isCallback(ts,node)?'callback':'function';
}
function parameterShape({ts,checker},parameter,location) {
  const declaration=parameter.valueDeclaration;
  return {type:checker.getTypeOfSymbolAtLocation(parameter,location),
    optional:!!(parameter.flags&ts.SymbolFlags.Optional)||!!declaration?.initializer,
    rest:!!declaration?.dotDotDotToken};
}
function sameType({ts,checker},left,right) {
  if((left.flags&ts.TypeFlags.Any)!==(right.flags&ts.TypeFlags.Any)) return false;
  return checker.isTypeAssignableTo(left,right)&&checker.isTypeAssignableTo(right,left);
}
function sameParameters(context,left,right,location) {
  if(left.parameters.length!==right.parameters.length) return false;
  return left.parameters.every((parameter,index)=>{
    const a=parameterShape(context,parameter,location);
    const b=parameterShape(context,right.parameters[index],location);
    return a.optional===b.optional&&a.rest===b.rest&&sameType(context,a.type,b.type);
  });
}
function memberDeclaration(ts,node) {
  const methods=[ts.SyntaxKind.MethodDeclaration,ts.SyntaxKind.MethodSignature];
  if(methods.includes(node.kind)) return node;
  const properties=[ts.SyntaxKind.PropertyDeclaration,ts.SyntaxKind.PropertyAssignment];
  return properties.includes(node.parent.kind)?node.parent:undefined;
}
function instanceMemberName(ts,node) {
  const declaration=memberDeclaration(ts,node);
  if(!declaration) return undefined;
  const isStatic=ts.getModifiers(declaration)?.some(modifier=>modifier.kind===ts.SyntaxKind.StaticKeyword);
  return isStatic?undefined:declaration.name;
}
function objectMember(ts,checker,node) {
  const name=instanceMemberName(ts,node);
  if(!name) return false;
  const member=checker.getSymbolAtLocation(name);
  const object=checker.resolveName('Object',undefined,ts.SymbolFlags.Type,false);
  if(!member||!object) return false;
  const base=checker.getDeclaredTypeOfSymbol(object).getProperty(member.getName());
  if(!base) return false;
  const signature=checker.getSignatureFromDeclaration(node);
  if(!signature) return false;
  const signatures=checker.getSignaturesOfType(checker.getTypeOfSymbolAtLocation(base,node),ts.SignatureKind.Call);
  return signatures.some(baseSignature=>sameParameters({ts,checker},signature,baseSignature,node));
}
function initializerScope(ts,node) {
  if(ts.isClassStaticBlockDeclaration(node)) return true;
  return ts.isPropertyDeclaration(node)&&!!node.initializer;
}
module.exports={callableKind,objectMember,initializerScope};
