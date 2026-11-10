'use strict';
const {FlowExpressions}=require('./flow-expressions.js');
class FlowStatements {
  constructor(source) {
    this.source=source;this.ts=source.ts;
    this.expressions=new FlowExpressions(source,node=>this.functionBody(node));
  }
  functionBody(node) {
    const defaults=this.parameterDefaults(node);
    const body=this.ts.isBlock(node.body)?this.block(node.body):[{kind:'return',span:this.source.span(node.body),value:this.expressions.read(node.body)}];
    return [...defaults,...body];
  }
  parameterDefaults(node) {
    return node.parameters.filter(parameter=>parameter.initializer).map(parameter=>({kind:'if',span:this.source.span(parameter),
      condition:{kind:'value',span:this.source.span(parameter.name),text:`Argument ${this.source.text(parameter.name)} is undefined`,calls:[]},
      then_branch:this.initializer(parameter),else_branch:[]}));
  }
  evaluate(node) {return {kind:'evaluate',span:this.source.span(node),value:this.expressions.read(node)};}
  initializer(node) {
    const span=this.source.span(node);
    const bind={kind:'action',span,text:`Bind ${this.source.text(node.name)}`,calls:[]};
    return node.initializer?[this.evaluate(node.initializer),bind]:[bind];
  }
  declarations(node) {return node.declarations.flatMap(declaration=>this.initializer(declaration));}
  block(node) {return this.statements(node.statements);}
  statements(nodes) {
    const statements=[];
    for(const statement of nodes) {
      const exhausted=this.source.limits.exhausted();
      statements.push(...this.read(statement));
      if(exhausted) break;
    }
    return statements;
  }
  list(node) {return node?this.read(node):[];}
  action(node,text=this.source.text(node)) {return {kind:'action',span:this.source.span(node),text,calls:this.source.calls(node)};}
  read(node,label=null) {
    return this.source.limits.run(()=>this.readStatement(node,label),reason=>[{kind:'unsupported',span:this.source.span(node),text:this.source.text(node),reason}]);
  }
  readStatement(node,label) {
    if(this.ts.isBlock(node)) return [{kind:'block',span:this.source.span(node),label,body:this.block(node)}];
    if(this.ts.isLabeledStatement(node)) return this.labeled(node);
    const kind=this.kind(node);
    const statement=kind?this[kind](node,label):this.simple(node);
    return Array.isArray(statement)?statement:[statement];
  }
  labeled(node) {
    const kind=this.kind(node.statement);
    const target=kind||this.ts.SyntaxKind[node.statement.kind];
    if(['loop','switch','Block'].includes(target)) return this.read(node.statement,node.label.text);
    return [{kind:'block',span:this.source.span(node),label:node.label.text,body:this.read(node.statement)}];
  }
  kind(node) {
    const names={IfStatement:'if',ForStatement:'loop',ForInStatement:'loop',ForOfStatement:'loop',WhileStatement:'loop',DoStatement:'loop',
      SwitchStatement:'switch',TryStatement:'try',ReturnStatement:'abrupt',ThrowStatement:'abrupt',BreakStatement:'abrupt',ContinueStatement:'abrupt'};
    return names[this.ts.SyntaxKind[node.kind]];
  }
  if(node) {
    return {kind:'if',span:this.source.span(node),condition:this.expressions.read(node.expression),then_branch:this.listBody(node.thenStatement),else_branch:this.listBody(node.elseStatement)};
  }
  listBody(node) {return !node?[]:this.ts.isBlock(node)?this.block(node):this.read(node);}
  loop(node,label) {
    const span=this.source.span(node),body=this.listBody(node.statement),update=[];
    const initial=[];
    let condition;
    if(this.ts.isForStatement(node)) {
      if(node.initializer) initial.push(...(this.ts.isVariableDeclarationList(node.initializer)?this.declarations(node.initializer):[this.evaluate(node.initializer)]));
      if(node.incrementor) update.push(this.evaluate(node.incrementor));
      condition=this.expressions.read(node.condition);
    } else if(this.ts.isForInStatement(node)||this.ts.isForOfStatement(node)) {
      initial.push(this.evaluate(node.expression));
      condition={kind:'value',span:this.source.span(node.expression),text:'More iteration values?',calls:[]};
      body.unshift(this.action(node.initializer,`Bind iteration value: ${this.source.text(node.initializer)}`));
    } else condition=this.expressions.read(node.expression);
    return [...initial,{kind:'loop',span,label,condition,body,update,test_first:!this.ts.isDoStatement(node)}];
  }
  switch(node,label) {
    const clauses=node.caseBlock.clauses;
    return {kind:'switch',span:this.source.span(node),label,subject:this.expressions.read(node.expression),cases:clauses.map((clause,index)=>({
      labels:clause.expression?[this.source.text(clause.expression)]:[],test:this.expressions.read(clause.expression),span:this.source.span(clause),is_default:this.ts.isDefaultClause(clause),falls_through:index<clauses.length-1,
      body:this.statements(clause.statements)}))};
  }
  try(node) {
    const clause=node.catchClause;
    return {kind:'try',span:this.source.span(node),resources:[],body:this.block(node.tryBlock),
      catches:clause?[{parameter:clause.variableDeclaration?this.source.text(clause.variableDeclaration):'error',catch_all:true,body:this.block(clause.block)}]:[],
      finally:node.finallyBlock?this.block(node.finallyBlock):[],cleanup:[]};
  }
  abrupt(node) {
    const span=this.source.span(node);
    if(this.ts.isReturnStatement(node)) return {kind:'return',span,value:this.expressions.read(node.expression)};
    if(this.ts.isThrowStatement(node)) return {kind:'throw',span,value:this.expressions.read(node.expression)};
    return {kind:this.ts.isBreakStatement(node)?'break':'continue',span,label:node.label?.text||null};
  }
  simple(node) {
    const {ts}=this;
    if(ts.isFunctionDeclaration(node)&&node.body) return [this.action(node,`Declare function ${node.name?.text||'<anonymous>'}`),this.expressions.region(node)];
    const reason=this.unsupported(node);
    if(reason) return {kind:'unsupported',span:this.source.span(node),text:this.source.text(node),reason};
    if(ts.isExpressionStatement(node)) return this.evaluate(node.expression);
    if(ts.isVariableStatement(node)) return this.declarations(node.declarationList);
    const regions=this.expressions.regions(node);
    return [this.action(node),...regions];
  }
  unsupported(node) {
    const {ts}=this;
    if(ts.isVariableStatement(node)&&(node.declarationList.flags&ts.NodeFlags.Using)) return 'Explicit resource-management declarations require implicit cleanup flow';
    if(ts.isClassDeclaration(node)) return 'Class initialization is not expanded in function flowcharts';
    if(ts.isWithStatement(node)) return 'Dynamic with-scope control flow is unavailable';
    return undefined;
  }
}
module.exports={FlowStatements};
