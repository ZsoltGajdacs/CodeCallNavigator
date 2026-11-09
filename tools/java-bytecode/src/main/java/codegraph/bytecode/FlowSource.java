package codegraph.bytecode;

import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.Tree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.TreePathScanner;
import com.sun.source.util.Trees;
import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;

/** Original source, attributed paths and UTF-16-to-UTF-8 spans for one compilation unit. */
final class FlowSource {
    final CompilationUnitTree unit;
    final JavacTask task;
    final Trees trees;
    final CompiledTypes compiled;
    final String text;
    final String file;
    final List<String> diagnostics = new ArrayList<>();
    private final int[] offsets;
    private int depth;
    private final Map<Tree, TreePath> paths = new IdentityHashMap<>();

    FlowSource(CompilationUnitTree unit, JavacTask task, CompiledTypes compiled) throws IOException {
        this.unit = unit; this.task = task; this.compiled = compiled; this.trees = Trees.instance(task);
        text = unit.getSourceFile().getCharContent(true).toString();
        file = Path.of(unit.getSourceFile().toUri()).toRealPath().toString();
        offsets = byteOffsets(text);
        new TreePathScanner<Void, Void>() {
            @Override public Void scan(Tree tree, Void unused) {
                if (tree != null) paths.put(tree, getCurrentPath() == null ? new TreePath(unit) : new TreePath(getCurrentPath(), tree));
                return super.scan(tree, unused);
            }
        }.scan(unit, null);
    }
    boolean enter() {
        if (depth >= 24) { unsupported("Source flow nesting exceeds 24 levels; nested region is opaque"); return false; }
        depth++;
        return true;
    }
    void leave() { depth--; }
    TreePath path(Tree tree) { return paths.get(tree); }
    long start(Tree tree) { return trees.getSourcePositions().getStartPosition(unit, tree); }
    long end(Tree tree) { return trees.getSourcePositions().getEndPosition(unit, tree); }
    boolean located(Tree tree) { return tree != null && start(tree) >= 0 && end(tree) >= start(tree) && end(tree) <= text.length(); }
    Map<String, Object> span(Tree tree) {
        if (!located(tree)) return FlowJson.object("start", 0, "end", 0);
        return FlowJson.object("start", offsets[(int) start(tree)], "end", offsets[(int) end(tree)]);
    }
    String text(Tree tree) { return located(tree) ? text.substring((int) start(tree), (int) end(tree)) : tree == null ? "" : tree.toString(); }
    void unsupported(String reason) { if (!diagnostics.contains(reason)) diagnostics.add(reason); }

    private static int[] byteOffsets(String text) {
        int[] offsets = new int[text.length() + 1];
        int bytes = 0;
        for (int index = 0; index < text.length();) {
            int point = text.codePointAt(index);
            int chars = Character.charCount(point);
            offsets[index] = bytes;
            if (chars == 2) offsets[index + 1] = bytes;
            bytes += point <= 0x7f ? 1 : point <= 0x7ff ? 2 : point <= 0xffff ? 3 : 4;
            index += chars;
            offsets[index] = bytes;
        }
        return offsets;
    }
}
