package codegraph.bytecode;

import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.io.BufferedWriter;
import java.io.IOException;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;

/** One flow source IR per attributed callable, before graph inclusion policy. */
final class SourceFlow {
    private final JavacTask task;
    private final CompiledTypes compiled;
    private final BufferedWriter writer;
    private final Trees trees;
    private final Map<CompilationUnitTree, FlowSource> sources = new IdentityHashMap<>();
    SourceFlow(JavacTask task, CompiledTypes compiled, BufferedWriter writer) {
        this.task = task; this.compiled = compiled; this.writer = writer; trees = Trees.instance(task);
    }
    void write(List<TreePath> methods, List<TreePath> lambdas) throws IOException {
        for (TreePath path : methods) method(path);
        for (TreePath path : lambdas) lambda(path);
    }
    private FlowSource source(TreePath path) throws IOException {
        var unit = path.getCompilationUnit();
        if (!sources.containsKey(unit)) sources.put(unit, new FlowSource(unit, task, compiled));
        var source = sources.get(unit);
        source.diagnostics.clear();
        return source;
    }
    private void method(TreePath path) throws IOException {
        var tree = (MethodTree) path.getLeaf();
        if (tree.getBody() == null || !(trees.getElement(path) instanceof ExecutableElement method)) return;
        var source = source(path);
        if (!source.located(tree)) return;
        var signature = SourceSignatures.method(method, task, compiled);
        var body = new FlowStatements(source).list(tree.getBody());
        var function = FlowJson.object("symbol", signature.key(), "file", source.file, "language", "java",
                "span", source.span(tree), "body", body, "diagnostics", source.diagnostics);
        ContextTsv.write(writer, "flow", signature.signature(), signature.returns(), FlowJson.encode(function), "", "", "", "", "");
    }
    private void lambda(TreePath path) throws IOException {
        var source = source(path);
        var lambda = (LambdaExpressionTree) path.getLeaf();
        if (!source.located(lambda)) return;
        var statements = new FlowStatements(source);
        var body = lambda.getBody() instanceof com.sun.source.tree.ExpressionTree expression
                ? List.of(FlowJson.object("kind", "return", "span", source.span(expression), "value", statements.expression(expression)))
                : statements.list(lambda.getBody());
        String owner = lambdaOwner(path);
        long startLine = source.unit.getLineMap().getLineNumber(source.start(lambda));
        long endLine = source.unit.getLineMap().getLineNumber(source.end(lambda) - 1);
        String symbol = "@lambda:" + owner + ":" + source.start(lambda);
        var function = FlowJson.object("symbol", symbol, "file", source.file, "language", "java", "span", source.span(lambda),
                "body", body, "diagnostics", source.diagnostics, "lambda_owner", owner, "lambda_start_line", startLine, "lambda_end_line", endLine);
        ContextTsv.write(writer, "flow", symbol, "", FlowJson.encode(function), "", "", "", "", "");
    }
    private String lambdaOwner(TreePath path) {
        for (var enclosing = path.getParentPath(); enclosing != null; enclosing = enclosing.getParentPath()) {
            if (trees.getElement(enclosing) instanceof javax.lang.model.element.TypeElement type) return task.getElements().getBinaryName(type).toString();
        }
        return "";
    }
}
