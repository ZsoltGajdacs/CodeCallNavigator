package codecallnavigator.bytecode;

import com.sun.source.tree.MethodInvocationTree;
import com.sun.source.tree.Tree;
import java.util.Map;
import javax.lang.model.element.ElementKind;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.util.Elements;

/** Compiler facts belong to explicit method bodies and final ordinary invocation Values only. */
final class FlowCompilerFacts {
    private FlowCompilerFacts() {}

    static String execution(FlowSource source, ExecutableElement method, Tree body) {
        return authoredMethod(source, method) && meaningful(source, body) ? "synchronous" : null;
    }

    static void invocation(FlowSource source, MethodInvocationTree tree, Map<String, Object> call) {
        var path = source.path(tree);
        if (path == null || !(source.trees.getElement(path) instanceof ExecutableElement method)) return;
        if (!authoredMethod(source, method) || !meaningful(source, tree)) return;
        if (!source.span(tree).equals(call.get("span"))) return;
        call.put("invocation", FlowJson.object("timing", "synchronous", "dispatch", direct(method) ? "direct" : "virtual",
                "isolated_ordered", true));
    }

    private static boolean authoredMethod(FlowSource source, ExecutableElement method) {
        return source.attribution.complete(source.unit) && method.getKind() == ElementKind.METHOD
                && source.task.getElements().getOrigin(method) == Elements.Origin.EXPLICIT;
    }

    private static boolean meaningful(FlowSource source, Tree tree) {
        return source.located(tree) && source.start(tree) < source.end(tree);
    }

    private static boolean direct(ExecutableElement method) {
        var modifiers = method.getModifiers();
        return modifiers.contains(Modifier.STATIC) || modifiers.contains(Modifier.PRIVATE)
                || modifiers.contains(Modifier.FINAL)
                || method.getEnclosingElement().getModifiers().contains(Modifier.FINAL);
    }
}
