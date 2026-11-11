package codecallnavigator.bytecode;

import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.net.URI;

/** Immutable source location retained before javac lowers or clears its trees. */
record LambdaSourceSite(String owner, URI file, long offset) {
    static LambdaSourceSite of(TreePath path, JavacTask task) {
        var trees = Trees.instance(task);
        var type = LambdaOwner.enclosing(path, trees);
        String owner = type == null ? "" : task.getElements().getBinaryName(type).toString();
        return new LambdaSourceSite(owner, path.getCompilationUnit().getSourceFile().toUri(),
                trees.getSourcePositions().getStartPosition(path.getCompilationUnit(), path.getLeaf()));
    }
    String symbol() { return "@lambda:" + owner + ":" + offset; }
}
