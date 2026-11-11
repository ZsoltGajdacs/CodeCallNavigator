package codecallnavigator.bytecode;

import com.sun.source.tree.*;
import com.sun.source.util.TreeScanner;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.TypeElement;

/** Expressions preserve conditional evaluation and keep callback bodies in deferred regions. */
final class FlowExpressions {
    private static final Set<String> REACTOR = Set.of("reactor.core.publisher.Mono", "reactor.core.publisher.Flux");
    private static final Set<String> OPERATORS = Set.of("defer", "map", "flatMap", "filter", "switchIfEmpty", "onErrorResume", "then", "zip", "when", "collectList");
    private final FlowSource source;
    private final FlowStatements statements;
    FlowExpressions(FlowSource source, FlowStatements statements) { this.source = source; this.statements = statements; }

    Map<String, Object> expression(Tree tree) {
        if (!source.enter()) return FlowJson.object("kind", "value", "span", source.span(tree), "text", "Opaque nested expression", "calls", List.of(), "regions", List.of());
        try { return parseExpression(tree); } finally { source.leave(); }
    }
    private Map<String, Object> parseExpression(Tree tree) {
        if (tree instanceof ParenthesizedTree parenthesized) return expression(parenthesized.getExpression());
        if (tree instanceof BinaryTree binary) {
            String kind = logicalKind(tree);
            if (kind != null) return FlowJson.object("kind", kind, "span", source.span(tree),
                    "left", expression(binary.getLeftOperand()), "right", expression(binary.getRightOperand()));
        }
        if (tree instanceof ConditionalExpressionTree conditional) return FlowJson.object("kind", "conditional", "span", source.span(tree),
                "condition", expression(conditional.getCondition()), "when_true", expression(conditional.getTrueExpression()), "when_false", expression(conditional.getFalseExpression()));
        if (tree instanceof SwitchExpressionTree selection) return FlowJson.object("kind", "switch", "span", source.span(tree),
                "subject", expression(selection.getExpression()), "cases", statements.cases(selection.getCases(), true));
        return operationExpression(tree);
    }
    private String logicalKind(Tree tree) {
        return switch (tree.getKind()) { case CONDITIONAL_AND -> "and"; case CONDITIONAL_OR -> "or"; default -> null; };
    }
    private Map<String, Object> operationExpression(Tree tree) {
        if (tree instanceof MethodInvocationTree invocation) return invocation(invocation);
        if (tree instanceof NewClassTree creation) return creation(creation);
        if (tree instanceof BinaryTree binary) return sequence(tree, List.of(expression(binary.getLeftOperand()), expression(binary.getRightOperand())), value(tree));
        if (tree instanceof AssignmentTree assignment) return sequence(tree, List.of(expression(assignment.getVariable()), expression(assignment.getExpression())), value(tree));
        if (tree instanceof CompoundAssignmentTree assignment) return sequence(tree, List.of(expression(assignment.getVariable()), expression(assignment.getExpression())), value(tree));
        if (tree instanceof TypeCastTree cast) return sequence(tree, List.of(expression(cast.getExpression())), value(tree));
        if (tree instanceof UnaryTree unary) return sequence(tree, List.of(expression(unary.getExpression())), value(tree));
        return remaining(tree);
    }
    private Map<String, Object> remaining(Tree tree) {
        if (tree instanceof ArrayAccessTree array) return sequence(tree, List.of(expression(array.getExpression()), expression(array.getIndex())), value(tree));
        if (tree instanceof InstanceOfTree test) return sequence(tree, List.of(expression(test.getExpression())), value(tree));
        if (tree instanceof MemberSelectTree member) return sequence(tree, receiver(member.getExpression()), value(tree));
        if (tree instanceof NewArrayTree array) {
            var parts = new ArrayList<Map<String, Object>>();
            for (ExpressionTree dimension : array.getDimensions()) parts.add(expression(dimension));
            if (array.getInitializers() != null) for (ExpressionTree initializer : array.getInitializers()) parts.add(expression(initializer));
            return sequence(tree, parts, value(tree));
        }
        return callback(tree, null);
    }
    private Map<String, Object> value(Tree tree) { return value(tree, List.of(), List.of()); }
    private Map<String, Object> value(Tree tree, List<Map<String, Object>> calls, List<Map<String, Object>> regions) {
        return FlowJson.object("kind", "value", "span", source.span(tree), "text", source.text(tree), "calls", calls, "regions", regions);
    }
    private Map<String, Object> sequence(Tree tree, List<Map<String, Object>> prefix, Map<String, Object> value) {
        if (prefix.isEmpty()) return value;
        var items = new ArrayList<>(prefix);
        items.add(value);
        return FlowJson.object("kind", "sequence", "span", source.span(tree), "items", items);
    }
    private List<Map<String, Object>> receiver(Tree tree) {
        var path = source.path(tree);
        if (path != null && source.trees.getElement(path) instanceof TypeElement) return List.of();
        return List.of(expression(tree));
    }
    private Map<String, Object> invocation(MethodInvocationTree tree) {
        var parts = new ArrayList<Map<String, Object>>();
        if (tree.getMethodSelect() instanceof MemberSelectTree member) parts.addAll(receiver(member.getExpression()));
        String operation = reactor(tree);
        for (ExpressionTree argument : tree.getArguments()) {
            parts.add(argument instanceof LambdaExpressionTree || argument instanceof MemberReferenceTree ? callback(argument, operation) : expression(argument));
        }
        var regions = new ArrayList<Map<String, Object>>();
        if (operation != null) regions.add(reactorStage(tree, operation));
        return sequence(tree, parts, value(tree, List.of(call(tree, false)), regions));
    }
    private Map<String, Object> creation(NewClassTree tree) {
        var parts = new ArrayList<Map<String, Object>>();
        if (tree.getEnclosingExpression() != null) parts.add(expression(tree.getEnclosingExpression()));
        for (ExpressionTree argument : tree.getArguments()) parts.add(expression(argument));
        if (tree.getClassBody() != null) source.unsupported("Anonymous class bodies are separate declarations, not constructor execution");
        return sequence(tree, parts, value(tree, List.of(call(tree, false)), List.of()));
    }
    private Map<String, Object> callback(Tree tree, String operation) {
        var collector = new Calls();
        collector.operation = operation;
        collector.scan(tree, null);
        var result = value(tree, collector.calls, collector.regions);
        if (tree instanceof MemberReferenceTree reference) return sequence(tree, receiver(reference.getQualifierExpression()), result);
        return result;
    }
    private Map<String, Object> reactorStage(MethodInvocationTree tree, String name) {
        if (OPERATORS.contains(name)) return region(tree, operationLabel(name), List.of());
        String reason = "Unrecognized compiler-confirmed Reactor operator " + name + " remains opaque";
        source.unsupported(reason);
        return region(tree, "Reactor " + name + " · opaque subscription stage", List.of(
                FlowJson.object("kind", "unsupported", "span", source.span(tree), "text", source.text(tree), "reason", reason)));
    }

    private ExecutableElement method(Tree tree) {
        var path = source.path(tree);
        return path != null && source.trees.getElement(path) instanceof ExecutableElement method ? method : null;
    }
    private Map<String, Object> call(Tree tree, boolean reference) {
        var method = method(tree);
        var targets = new ArrayList<Map<String, Object>>();
        String diagnostic = null;
        if (method == null) diagnostic = "Compiler did not resolve this call target";
        else {
            try {
                String certainty = reference ? "reference" : direct(method) ? "direct" : "dispatch";
                targets.add(FlowJson.object("symbol", SourceSignatures.method(method, source.task, source.compiled).key(),
                        "certainty", certainty, "provenance", "java.javac.source-call"));
            } catch (IOException error) { diagnostic = "Compiler target signature is unavailable: " + error.getMessage(); }
        }
        return FlowJson.object("span", source.span(tree), "label", source.text(tree), "targets", targets, "diagnostic", diagnostic);
    }
    private boolean direct(ExecutableElement method) {
        return method.getModifiers().contains(Modifier.STATIC) || method.getModifiers().contains(Modifier.PRIVATE)
                || method.getModifiers().contains(Modifier.FINAL) || method.getSimpleName().contentEquals("<init>")
                || method.getEnclosingElement().getModifiers().contains(Modifier.FINAL);
    }
    private String reactor(MethodInvocationTree tree) {
        var method = method(tree);
        if (method == null || !(method.getEnclosingElement() instanceof TypeElement owner)) return null;
        return REACTOR.contains(owner.getQualifiedName().toString()) ? method.getSimpleName().toString() : null;
    }
    private Map<String, Object> region(Tree tree, String label, List<Map<String, Object>> body) {
        return FlowJson.object("kind", "region", "span", source.span(tree), "label", label, "deferred", true, "body", body);
    }
    private String operationLabel(String operation) {
        return switch (operation) {
            case "switchIfEmpty" -> "Reactor switchIfEmpty · empty subscription alternative";
            case "onErrorResume" -> "Reactor onErrorResume · error subscription alternative";
            case "zip", "when" -> "Reactor " + operation + " · sources may execute concurrently on subscription";
            case "filter" -> "Reactor filter · predicate may complete empty";
            case "then" -> "Reactor then · subscribe after source completion";
            case "collectList" -> "Reactor collectList · emit collected elements on completion";
            default -> "Reactor " + operation + " · deferred subscription stage";
        };
    }

    private final class Calls extends TreeScanner<Void, Void> {
        final List<Map<String, Object>> calls = new ArrayList<>();
        final List<Map<String, Object>> regions = new ArrayList<>();
        private String operation;
        @Override public Void visitLambdaExpression(LambdaExpressionTree tree, Void unused) {
            String label = operation == null ? "Callback · deferred invocation" : operationLabel(operation) + " callback";
            List<Map<String, Object>> body = tree.getBody() instanceof ExpressionTree expression
                    ? List.of(FlowJson.object("kind", "return", "span", source.span(expression), "value", expression(expression)))
                    : statements.list(tree.getBody());
            regions.add(region(tree, label, body));
            return null;
        }
        @Override public Void visitMemberReference(MemberReferenceTree tree, Void unused) {
            String label = operation == null ? "Method reference · deferred invocation" : operationLabel(operation) + " method reference";
            regions.add(region(tree, label, List.of(FlowJson.object("kind", "action", "span", source.span(tree),
                    "text", source.text(tree), "calls", List.of(call(tree, true))))));
            return null;
        }
        @Override public Void visitClass(ClassTree tree, Void unused) { return null; }
    }
}
