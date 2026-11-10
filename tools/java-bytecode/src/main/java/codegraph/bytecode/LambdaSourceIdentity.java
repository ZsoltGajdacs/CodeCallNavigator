package codegraph.bytecode;

import java.io.IOException;

import com.sun.source.tree.BlockTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.tree.VariableTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.IdentityHashMap;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.NestingKind;
import javax.lang.model.element.TypeElement;
import javax.lang.model.element.VariableElement;

/** Portable source keys are exported only after Rust proves source-to-bytecode attribution. */
final class LambdaSourceIdentity {
    private final JavacTask task;
    private final CompiledTypes compiled;
    private final Trees trees;
    private final Map<Object, String> keys = new IdentityHashMap<>();
    private final Map<String, Integer> occurrences = new HashMap<>();
    LambdaSourceIdentity(JavacTask task, CompiledTypes compiled) {
        this.task = task; this.compiled = compiled; trees = Trees.instance(task);
    }
    String key(TreePath path) throws IOException {
        var tree = path.getLeaf();
        if (keys.containsKey(tree)) return keys.get(tree);
        if (!stableTypes(path)) return "";
        String enclosing = enclosing(path);
        if (enclosing.isEmpty()) return "";
        String base = FlowJson.encode(FlowJson.object("enclosing", enclosing,
                "role", LambdaSourceRole.role(path), "syntax", fingerprint(tree.toString())));
        String group = path.getCompilationUnit().getSourceFile().toUri() + ":" + base;
        int occurrence = occurrences.getOrDefault(group, 0);
        occurrences.put(group, occurrence + 1);
        String key = FlowJson.encode(FlowJson.object("source_lambda", base, "occurrence", occurrence));
        keys.put(tree, key);
        return key;
    }
    String method(ExecutableElement method) throws IOException { return SourceSignatures.method(method, task, compiled).key(); }
    private boolean stableTypes(TreePath path) {
        for (var enclosing = path.getParentPath(); enclosing != null; enclosing = enclosing.getParentPath()) {
            var element = trees.getElement(enclosing);
            if (element instanceof TypeElement type && unstable(type.getNestingKind())) return false;
        }
        return true;
    }
    private boolean unstable(NestingKind kind) { return kind == NestingKind.ANONYMOUS || kind == NestingKind.LOCAL; }
    private String enclosing(TreePath path) throws IOException {
        for (var enclosing = path.getParentPath(); enclosing != null; enclosing = enclosing.getParentPath()) {
            String owner = directOwner(enclosing);
            if (!owner.isEmpty()) return owner;
        }
        return "";
    }
    private String directOwner(TreePath path) throws IOException {
        var tree = path.getLeaf();
        if (tree instanceof LambdaExpressionTree) return key(path);
        var element = trees.getElement(path);
        if (tree instanceof MethodTree && element instanceof ExecutableElement method) return method(method);
        return initializer(path, element);
    }
    private String initializer(TreePath path, javax.lang.model.element.Element element) {
        if (!(path.getLeaf() instanceof VariableTree)) return blockOwner(path);
        if (!(element instanceof VariableElement variable)) return "";
        if (!(variable.getEnclosingElement() instanceof TypeElement owner)) return "";
        return task.getElements().getBinaryName(owner) + ":field:" + variable.getSimpleName();
    }
    private String blockOwner(TreePath path) {
        if (!(path.getLeaf() instanceof BlockTree block)) return "";
        var parent = path.getParentPath();
        if (parent == null || !(parent.getLeaf() instanceof com.sun.source.tree.ClassTree)) return "";
        if (!(trees.getElement(parent) instanceof TypeElement owner)) return "";
        return task.getElements().getBinaryName(owner) + (block.isStatic() ? ":static" : ":instance");
    }
    private String fingerprint(String normalizedSyntax) {
        try {
            var digest = MessageDigest.getInstance("SHA-256").digest(normalizedSyntax.getBytes(StandardCharsets.UTF_8));
            return HexFormat.of().formatHex(digest);
        } catch (NoSuchAlgorithmException impossible) { throw new IllegalStateException(impossible); }
    }
}
