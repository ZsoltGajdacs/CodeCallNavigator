package codegraph.bytecode;

import com.sun.source.tree.*;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/** Detached Java statements. Completion semantics belong to the shared pure builder. */
final class FlowStatements {
    private final FlowSource source;
    private final FlowExpressions expressions;
    FlowStatements(FlowSource source) { this.source = source; expressions = new FlowExpressions(source, this); }

    List<Map<String, Object>> list(Tree tree) {
        if (tree == null) return List.of();
        if (!source.enter()) return List.of(unsupported(tree, "Source flow nesting exceeds 24 levels"));
        try { return parseList(tree); } finally { source.leave(); }
    }
    private List<Map<String, Object>> parseList(Tree tree) {
        if (tree instanceof BlockTree block) {
            var result = new ArrayList<Map<String, Object>>();
            for (StatementTree statement : block.getStatements()) result.addAll(list(statement));
            return result;
        }
        if (!source.located(tree)) return List.of();
        return List.of(statement(tree));
    }
    Map<String, Object> expression(Tree tree) { return expressions.expression(tree); }
    private Map<String, Object> node(String kind, Tree tree, Object... fields) {
        var result = FlowJson.object("kind", kind, "span", source.span(tree));
        result.putAll(FlowJson.object(fields));
        return result;
    }
    private Map<String, Object> statement(Tree tree) {
        return switch (tree.getKind()) {
            case IF -> branch((IfTree) tree);
            case FOR_LOOP -> forLoop((ForLoopTree) tree);
            case ENHANCED_FOR_LOOP -> foreach((EnhancedForLoopTree) tree);
            case WHILE_LOOP -> whileLoop((WhileLoopTree) tree);
            case DO_WHILE_LOOP -> doLoop((DoWhileLoopTree) tree);
            case SWITCH -> switchStatement((SwitchTree) tree);
            case TRY -> tryStatement((TryTree) tree);
            default -> abrupt(tree);
        };
    }
    private Map<String, Object> abrupt(Tree tree) {
        return switch (tree.getKind()) {
            case RETURN -> node("return", tree, "value", nullable(((ReturnTree) tree).getExpression()));
            case THROW -> node("throw", tree, "value", expression(((ThrowTree) tree).getExpression()));
            case YIELD -> node("yield", tree, "value", nullable(((YieldTree) tree).getValue()));
            case BREAK -> node("break", tree, "label", label(((BreakTree) tree).getLabel()));
            case CONTINUE -> node("continue", tree, "label", label(((ContinueTree) tree).getLabel()));
            case LABELED_STATEMENT -> labeled((LabeledStatementTree) tree);
            default -> simple(tree);
        };
    }
    private Map<String, Object> simple(Tree tree) {
        return switch (tree.getKind()) {
            case EXPRESSION_STATEMENT -> node("evaluate", tree, "value", expression(((ExpressionStatementTree) tree).getExpression()));
            case VARIABLE -> variable((VariableTree) tree);
            case EMPTY_STATEMENT -> node("action", tree, "text", ";", "calls", List.of());
            case CLASS -> node("action", tree, "text", "Local type declaration", "calls", List.of());
            case SYNCHRONIZED -> unsupported(tree, "Monitor acquisition and release are not modelled");
            case ASSERT -> unsupported(tree, "Assertion execution depends on the runtime assertion flag");
            default -> unsupported(tree, "Unsupported Java statement " + tree.getKind());
        };
    }
    private Map<String, Object> nullable(Tree tree) { return tree == null ? null : expression(tree); }
    private String label(javax.lang.model.element.Name name) { return name == null ? null : name.toString(); }
    private Map<String, Object> branch(IfTree tree) {
        return node("if", tree, "condition", expression(tree.getCondition()),
                "then_branch", list(tree.getThenStatement()), "else_branch", list(tree.getElseStatement()));
    }
    private Map<String, Object> variable(VariableTree tree) {
        var body = new ArrayList<Map<String, Object>>();
        if (tree.getInitializer() != null) body.add(node("evaluate", tree.getInitializer(), "value", expression(tree.getInitializer())));
        body.add(node("action", tree, "text", "Bind " + tree.getName(), "calls", List.of()));
        return node("block", tree, "body", body);
    }
    private Map<String, Object> forLoop(ForLoopTree tree) {
        var body = new ArrayList<Map<String, Object>>();
        for (StatementTree initializer : tree.getInitializer()) body.addAll(list(initializer));
        var update = new ArrayList<Map<String, Object>>();
        for (ExpressionStatementTree expression : tree.getUpdate()) update.addAll(list(expression));
        body.add(node("loop", tree, "condition", nullable(tree.getCondition()), "body", list(tree.getStatement()), "update", update, "test_first", true));
        return node("block", tree, "body", body);
    }
    private Map<String, Object> foreach(EnhancedForLoopTree tree) {
        var body = new ArrayList<Map<String, Object>>();
        body.add(node("action", tree.getVariable(), "text", "Next element → " + tree.getVariable().getName(), "calls", List.of()));
        body.addAll(list(tree.getStatement()));
        var condition = FlowJson.object("kind", "value", "span", source.span(tree.getExpression()),
                "text", "More elements in " + source.text(tree.getExpression()), "calls", List.of(), "regions", List.of());
        return node("block", tree, "body", List.of(node("evaluate", tree.getExpression(), "value", expression(tree.getExpression())),
                node("loop", tree, "condition", condition, "body", body, "update", List.of(), "test_first", true)));
    }
    private Map<String, Object> whileLoop(WhileLoopTree tree) {
        return node("loop", tree, "condition", expression(tree.getCondition()), "body", list(tree.getStatement()), "update", List.of(), "test_first", true);
    }
    private Map<String, Object> doLoop(DoWhileLoopTree tree) {
        return node("loop", tree, "condition", expression(tree.getCondition()), "body", list(tree.getStatement()), "update", List.of(), "test_first", false);
    }
    private Map<String, Object> labeled(LabeledStatementTree tree) {
        var nested = list(tree.getStatement());
        String label = tree.getLabel().toString();
        if (loopOrSwitch(tree.getStatement()) && !nested.isEmpty()) {
            var statement = nested.getFirst();
            statement.put("label", label);
            return statement;
        }
        if (forStatement(tree.getStatement()) && !nested.isEmpty()) {
            var statement = nested.getFirst();
            @SuppressWarnings("unchecked") var body = (List<Map<String, Object>>) statement.get("body");
            body.getLast().put("label", label);
            return statement;
        }
        return node("block", tree, "label", label, "body", nested);
    }
    private boolean loopOrSwitch(Tree tree) { return tree instanceof WhileLoopTree || tree instanceof DoWhileLoopTree || tree instanceof SwitchTree; }
    private boolean forStatement(Tree tree) { return tree instanceof ForLoopTree || tree instanceof EnhancedForLoopTree; }
    private Map<String, Object> switchStatement(SwitchTree tree) {
        return node("switch", tree, "subject", expression(tree.getExpression()), "cases", cases(tree.getCases(), false));
    }
    List<Map<String, Object>> cases(List<? extends CaseTree> cases, boolean expression) {
        var result = new ArrayList<Map<String, Object>>();
        for (CaseTree branch : cases) result.add(switchCase(branch, expression));
        return result;
    }
    private Map<String, Object> switchCase(CaseTree branch, boolean isExpression) {
        var labels = branch.getLabels().stream().map(Object::toString).toList();
        boolean isDefault = branch.getLabels().stream().anyMatch(label -> label.getKind() == Tree.Kind.DEFAULT_CASE_LABEL);
        var body = new ArrayList<Map<String, Object>>();
        if (branch.getGuard() != null) body.add(unsupported(branch.getGuard(), "Pattern guards are opaque; case dispatch is conservative"));
        if (branch.getCaseKind() == CaseTree.CaseKind.STATEMENT) {
            for (StatementTree statement : branch.getStatements()) body.addAll(list(statement));
        } else if (branch.getBody() instanceof ExpressionTree expression) {
            body.add(node(isExpression ? "yield" : "evaluate", expression, "value", expression(expression)));
        } else body.addAll(list(branch.getBody()));
        return FlowJson.object("span", source.span(branch), "labels", labels, "is_default", isDefault,
                "falls_through", branch.getCaseKind() == CaseTree.CaseKind.STATEMENT, "body", body);
    }
    private Map<String, Object> tryStatement(TryTree tree) {
        var resources = new ArrayList<Map<String, Object>>();
        var cleanup = new ArrayList<Map<String, Object>>();
        for (Tree resource : tree.getResources()) {
            if (resource instanceof VariableTree) resources.addAll(list(resource));
            else resources.add(node("evaluate", resource, "value", expression(resource)));
            String name = resource instanceof VariableTree variable ? variable.getName().toString() : source.text(resource);
            cleanup.addFirst(node("action", resource, "text", "Close resource " + name + " (if acquired)", "calls", List.of()));
        }
        var catches = tree.getCatches().stream().map(this::catchClause).toList();
        if (!resources.isEmpty()) source.unsupported("Resource initialization failures and suppressed exceptions use conservative cleanup paths");
        if (!catches.isEmpty()) source.unsupported("Catch dispatch and implicit exceptions are conservative; exception types are not proven");
        return node("try", tree, "resources", resources, "body", list(tree.getBlock()), "catches", catches,
                "finally", list(tree.getFinallyBlock()), "cleanup", cleanup);
    }
    private Map<String, Object> catchClause(CatchTree handler) {
        var element = source.trees.getElement(source.path(handler.getParameter()));
        boolean catchAll = element != null && element.asType().toString().equals("java.lang.Throwable");
        return FlowJson.object("parameter", source.text(handler.getParameter()), "body", list(handler.getBlock()), "catch_all", catchAll);
    }
    private Map<String, Object> unsupported(Tree tree, String reason) {
        source.unsupported(reason);
        return node("unsupported", tree, "text", source.text(tree), "reason", reason);
    }
}
