'use strict';
const data=require('./callable-data.js');
const syntax=require('./observer-syntax.js');
const {declarationKey}=require('./paths.js');
class Returns {
  constructor(flow) {this.flow=flow;this.bindings=flow.bindings;this.ts=flow.bindings.ts;this.summaries=new Map();}
  expression(declaration) {
    if(this.summaries.has(declaration)) return this.summaries.get(declaration);
    const result=this.summary(declaration);
    this.summaries.set(declaration,result);return result;
  }
  summary(node) {
    if(!node.body||!this.synchronous(node)) return undefined;
    if(!node.parameters.every(parameter=>this.simpleParameter(parameter))) return undefined;
    if(!this.ts.isBlock(node.body)) return node.body;
    return this.block(node.body);
  }
  synchronous(node) {return !node.asteriskToken&&!syntax.hasModifier(this.ts,node,this.ts.SyntaxKind.AsyncKeyword);}
  block(body) {
    const statements=body.statements,last=statements.at(-1);
    if(!last) return undefined;
    if(!this.ts.isReturnStatement(last)) return undefined;
    if(!last.expression) return undefined;
    if(!statements.slice(0,-1).every(statement=>this.aliases(statement))) return undefined;
    return last.expression;
  }
  aliases(statement) {
    if(!this.ts.isVariableStatement(statement)||!(statement.declarationList.flags&this.ts.NodeFlags.Const)) return false;
    return statement.declarationList.declarations.every(declaration=>this.ts.isIdentifier(declaration.name));
  }
  simpleParameter(parameter) {
    return this.ts.isIdentifier(parameter.name)&&!parameter.dotDotDotToken&&!this.flow.writes.has(declarationKey(parameter));
  }
  resolve(checker,node,context,seen) {
    if(node.questionDotToken) return data.unknown('optional callable return wrapper');
    if(node.arguments.some(argument=>this.ts.isSpreadElement(argument))) return data.unknown('spread callable return arguments');
    const callee=this.flow.values.resolve(checker,node.expression,context,seen);
    if(callee.unsafe||!callee.targets.size) return data.unknown('unsupported callable return wrapper');
    const values=[...callee.targets.keys()].map(index=>this.call(checker,node,index,{context,seen}));
    return values.some(value=>value.unsafe)?values.find(value=>value.unsafe):data.merge(values);
  }
  call(checker,node,index,{context,seen}) {
    const declaration=this.bindings.byIndex.get(index),expression=declaration&&this.expression(declaration);
    if(!expression) return data.unknown('unsupported callable return wrapper');
    const environment=this.environment(checker,node,declaration,{context,seen,index});
    const ownerChecker=this.flow.checkerFor(declaration,checker);
    const result=this.flow.values.resolve(ownerChecker,expression,{owner:declaration,returning:declaration,environment},seen);
    return data.derived(result,{kind:'return',site:node,support:index});
  }
  environment(checker,node,declaration,{context,seen,index}) {
    const environment=new Map();
    const ownerChecker=this.flow.checkerFor(declaration,checker);
    for(const [position,parameter] of declaration.parameters.entries()) {
      const argument=node.arguments[position];
      let value;
      if(argument) value=this.argument(checker,argument,parameter,{context,seen});
      else if(parameter.initializer) value=this.flow.values.resolve(ownerChecker,parameter.initializer,{owner:declaration,returning:declaration,environment},seen);
      else value=data.value({nullish:true});
      environment.set(declarationKey(parameter),data.derived(value,{kind:'parameter',site:argument||node,support:index}));
    }
    return environment;
  }
  argument(checker,node,parameter,{context,seen}) {
    const value=this.flow.values.resolve(checker,node,context,seen);
    if(value.unsafe) return value;
    if(!parameter.initializer) return value;
    return value.nullish?data.unknown('explicit nullish callable default argument'):value;
  }
}
module.exports={Returns};
