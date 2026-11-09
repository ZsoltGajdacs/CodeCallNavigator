'use strict';
const {isCallable}=require('./syntax.js');
const {opaqueExpression}=require('./flow-operators.js');
class FlowExpressions {
  constructor(source,body) {this.source=source;this.ts=source.ts;this.body=body;}
  value(node) {
    return {kind:'value',text:this.source.text(node),span:this.source.span(node),calls:this.source.calls(node),regions:this.regions(node)};
  }
  read(node) {
    if(!node) return null;
    return this.source.limits.run(()=>this.readExpression(node),()=>({kind:'value',text:this.source.text(node),span:this.source.span(node),calls:[],regions:[]}));
  }
  readExpression(node) {
    const {ts}=this;
    if(opaqueExpression(ts,node)) return this.value(node);
    if(ts.isParenthesizedExpression(node)) return this.read(node.expression);
    const span=this.source.span(node);
    if(ts.isConditionalExpression(node)) return {kind:'conditional',span,condition:this.read(node.condition),when_true:this.read(node.whenTrue),when_false:this.read(node.whenFalse)};
    const kind=this.binaryKind(node);
    if(kind) return {kind,span,left:this.read(node.left),right:this.read(node.right)};
    if(this.hasNestedBranch(node)) return this.sequence(node);
    return this.value(node);
  }
  binaryKind(node) {
    if(!this.ts.isBinaryExpression(node)) return undefined;
    const kind=node.operatorToken.kind;
    if(kind===this.ts.SyntaxKind.AmpersandAmpersandToken) return 'and';
    if(kind===this.ts.SyntaxKind.BarBarToken) return 'or';
    return undefined;
  }
  hasNestedBranch(node) {
    if(isCallable(this.ts,node)) return false;
    const children=this.operands(node);
    return children.some(child=>this.branching(child));
  }
  branching(node) {
    if(opaqueExpression(this.ts,node)) return false;
    if(isCallable(this.ts,node)) return false;
    if(this.ts.isConditionalExpression(node)||this.binaryKind(node)) return true;
    return this.operands(node).some(child=>this.branching(child));
  }
  operands(node) {
    const call=node=>[node.expression,...node.arguments||[]];
    const template=node=>node.templateSpans.map(span=>span.expression);
    const operands={CallExpression:call,NewExpression:call,BinaryExpression:node=>[node.left,node.right],
      PropertyAccessExpression:node=>[node.expression],ElementAccessExpression:node=>[node.expression,node.argumentExpression],
      ArrayLiteralExpression:node=>[...node.elements],ObjectLiteralExpression:node=>node.properties.flatMap(property=>this.propertyOperands(property)),
      TemplateExpression:template,TaggedTemplateExpression:node=>[node.tag,...this.templateOperands(node.template)],
      PrefixUnaryExpression:node=>[node.operand],PostfixUnaryExpression:node=>[node.operand],
      JsxElement:node=>[node.openingElement,...node.children],JsxFragment:node=>[...node.children],
      JsxSelfClosingElement:node=>[node.attributes],JsxOpeningElement:node=>[node.attributes],
      JsxAttributes:node=>[...node.properties],JsxAttribute:node=>node.initializer?[node.initializer]:[]};
    const read=operands[this.ts.SyntaxKind[node.kind]];
    if(read) return read(node);
    return node.expression?[node.expression]:[];
  }
  templateOperands(node) {return this.ts.isTemplateExpression(node)?node.templateSpans.map(span=>span.expression):[];}
  propertyOperands(node) {
    const items=[];
    if(node.name&&this.ts.isComputedPropertyName(node.name)) items.push(node.name.expression);
    if(node.initializer) items.push(node.initializer);
    else if(node.expression) items.push(node.expression);
    else if(isCallable(this.ts,node)) items.push(node);
    return items;
  }
  sequence(node) {
    const final={kind:'value',text:this.source.text(node),span:this.source.span(node),calls:this.source.ownCalls(node),regions:[]};
    return {kind:'sequence',span:this.source.span(node),items:[...this.operands(node).map(child=>this.read(child)),final]};
  }
  regions(node) {
    const regions=[];
    const visit=current=>{
      if(isCallable(this.ts,current)) {
        if(current.body) regions.push(this.region(current));
        return;
      }
      this.ts.forEachChild(current,visit);
    };
    visit(node);return regions;
  }
  region(node) {
    const index=this.source.bindings.index(node);
    const label=this.source.bindings.graph.nodes[index]?.label||'Deferred callback';
    return {kind:'region',span:this.source.span(node),label:`Separate function body: ${label}`,deferred:true,body:this.body(node)};
  }
}
module.exports={FlowExpressions};
