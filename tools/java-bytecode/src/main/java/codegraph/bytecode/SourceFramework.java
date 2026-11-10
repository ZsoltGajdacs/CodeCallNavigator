package codegraph.bytecode;

import com.sun.source.tree.ClassTree;
import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.MethodInvocationTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.TreeScanner;
import java.io.BufferedWriter;
import java.io.IOException;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;

/** Two compiler-owned passes: listeners, then publisher sites in the same source context. */
final class SourceFramework {
    private final SpringTypes types;
    private final CompiledTypes compiled;
    private final BufferedWriter writer;
    private final Map<CompilationUnitTree, FlowSource> sources = new IdentityHashMap<>();
    SourceFramework(JavacTask task, CompiledTypes compiled, BufferedWriter writer) {
        types = new SpringTypes(task); this.compiled = compiled; this.writer = writer;
    }
    void write(List<TreePath> methods) throws IOException {
        if (types.external(SpringTypes.PUBLISHER) == null) return;
        write("framework-diagnostic", "Spring event analysis is limited to compiler-qualified method publishers and source listeners in the same source context; lambda publishers, custom composed annotations and cross-context wiring remain unresolved.");
        var listeners = new SpringListeners(types).collect(methods);
        for (var listener : listeners) {
            write("framework-listener", SourceSignatures.method(listener.method(), types.task, compiled).key());
        }
        for (var path : methods) {
            if (types.trees.getElement(path) instanceof ExecutableElement method) scan(path, method, listeners);
        }
    }
    private FlowSource source(TreePath path) throws IOException {
        var unit = path.getCompilationUnit();
        if (!sources.containsKey(unit)) sources.put(unit, new FlowSource(unit, types.task, compiled));
        return sources.get(unit);
    }
    private void scan(TreePath path, ExecutableElement method, List<SpringListeners.Listener> listeners) throws IOException {
        var body = ((MethodTree) path.getLeaf()).getBody();
        if (body == null) return;
        var source = source(path);
        var caller = SourceSignatures.method(method, types.task, compiled).key();
        var failure = new IOException[1];
        new TreeScanner<Void, Void>() {
            @Override public Void visitClass(ClassTree tree, Void unused) { return null; }
            @Override public Void visitLambdaExpression(LambdaExpressionTree tree, Void unused) { return null; }
            @Override public Void visitMethodInvocation(MethodInvocationTree tree, Void unused) {
                try { invocation(source, caller, tree, listeners); }
                catch (IOException error) { failure[0] = error; }
                return super.visitMethodInvocation(tree, unused);
            }
        }.scan(body, null);
        if (failure[0] != null) throw failure[0];
    }
    private void invocation(FlowSource source, String caller, MethodInvocationTree call, List<SpringListeners.Listener> listeners) throws IOException {
        if (!(types.trees.getElement(source.path(call)) instanceof ExecutableElement method)) return;
        if (!method.getSimpleName().contentEquals("publishEvent") || call.getArguments().size() != 1) return;
        if (!types.is(method.getEnclosingElement().asType(), SpringTypes.PUBLISHER)) return;
        var payload = types.trees.getTypeMirror(source.path(call.getArguments().getFirst()));
        var events = types.events(payload);
        if (events.isEmpty()) {
            write("framework-diagnostic", "Spring publishEvent payload is erased, generic or unresolved at " + source.file + ":" + source.unit.getLineMap().getLineNumber(source.start(call)));
            return;
        }
        for (var listener : listeners) {
            boolean matches = listener.accepted().stream().anyMatch(accepted -> events.stream().anyMatch(event -> types.assignable(event, accepted)));
            if (matches) pair(source, caller, call, listener);
        }
    }
    private void pair(FlowSource source, String caller, MethodInvocationTree call, SpringListeners.Listener listener) throws IOException {
        var registration = source(listener.path());
        var data = FlowJson.object("caller", caller,
                "handler", SourceSignatures.method(listener.method(), types.task, compiled).key(),
                "invocation", site(source, call), "registration", site(registration, listener.path().getLeaf()),
                "event_type", types.trees.getTypeMirror(source.path(call.getArguments().getFirst())).toString(), "semantics", listener.semantics(), "qualifiers", listener.qualifiers());
        write("framework", FlowJson.encode(data));
    }
    private Map<String, Object> site(FlowSource source, com.sun.source.tree.Tree tree) {
        long start = source.start(tree);
        return FlowJson.object("file", source.file, "line", source.unit.getLineMap().getLineNumber(start),
                "column", source.unit.getLineMap().getColumnNumber(start), "span", source.span(tree));
    }
    private void write(String kind, String json) throws IOException {
        ContextTsv.write(writer, kind, "", "", json, "", "", "", "", "");
    }
}
