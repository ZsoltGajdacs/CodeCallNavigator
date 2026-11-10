package codegraph.bytecode;

import com.sun.source.tree.MethodInvocationTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TaskEvent;
import com.sun.source.util.TaskListener;
import com.sun.source.util.TreeScanner;
import com.sun.source.util.Trees;
import com.sun.tools.javac.code.Symbol.DynamicMethodSymbol;
import com.sun.tools.javac.code.Symbol.MethodHandleSymbol;
import com.sun.tools.javac.tree.JCTree.JCFieldAccess;
import java.util.HashMap;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;

/** Captures javac's actual bootstrap implementation handle at each original lambda position. */
final class LambdaLowering implements TaskListener {
    private final JavacTask task;
    private final CompiledTypes compiled;
    private final Map<LambdaSourceSite, String> implementations = new HashMap<>();
    private String failure = "";

    LambdaLowering(JavacTask task, CompiledTypes compiled) {
        this.task = task;
        this.compiled = compiled;
    }
    Map<LambdaSourceSite, String> implementations() { return Map.copyOf(implementations); }
    String failure() { return failure; }

    @Override public void started(TaskEvent event) {
        if (event.getKind() != TaskEvent.Kind.GENERATE) return;
        new Sites(event).scan(event.getCompilationUnit(), null);
    }

    private final class Sites extends TreeScanner<Void, Void> {
        private final TaskEvent event;
        private final Trees trees = Trees.instance(task);
        Sites(TaskEvent event) { this.event = event; }
        @Override public Void visitMethodInvocation(MethodInvocationTree call, Void unused) {
            if (call.getMethodSelect() instanceof JCFieldAccess access
                    && access.sym instanceof DynamicMethodSymbol dynamic) capture(call, dynamic);
            return super.visitMethodInvocation(call, unused);
        }
        private void capture(MethodInvocationTree call, DynamicMethodSymbol dynamic) {
            String bootstrapOwner = dynamic.bsm.owner.toString();
            if (!bootstrapOwner.equals("java.lang.invoke.LambdaMetafactory")) return;
            Object[] arguments = dynamic.staticArgs;
            if (arguments.length < 3 || !(arguments[1] instanceof MethodHandleSymbol handle)) return;
            long offset = trees.getSourcePositions().getStartPosition(event.getCompilationUnit(), call);
            try {
                var method = (ExecutableElement) handle.baseSymbol();
                var owner = (javax.lang.model.element.TypeElement) method.getEnclosingElement();
                String binary = task.getElements().getBinaryName(owner).toString();
                var site = new LambdaSourceSite(binary, event.getCompilationUnit().getSourceFile().toUri(), offset);
                String key = SourceSignatures.method(method, task, compiled).key();
                String previous = implementations.putIfAbsent(site, key);
                if (previous != null && !previous.equals(key)) failure = "Conflicting lowered implementation handles";
            } catch (java.io.IOException | RuntimeException error) {
                failure = "Cannot resolve lowered implementation handle";
            }
        }
    }
}
