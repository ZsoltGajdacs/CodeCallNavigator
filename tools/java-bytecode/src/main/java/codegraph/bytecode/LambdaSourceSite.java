package codegraph.bytecode;

import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.net.URI;
import javax.lang.model.element.TypeElement;

/** Immutable source location retained before javac lowers or clears its trees. */
record LambdaSourceSite(String owner, URI file, long offset) {
    static LambdaSourceSite of(TreePath path, JavacTask task) {
        var trees = Trees.instance(task);
        String owner = "";
        for (var parent = path.getParentPath(); parent != null; parent = parent.getParentPath()) {
            if (trees.getElement(parent) instanceof TypeElement type) {
                owner = task.getElements().getBinaryName(type).toString();
                break;
            }
        }
        return new LambdaSourceSite(owner, path.getCompilationUnit().getSourceFile().toUri(),
                trees.getSourcePositions().getStartPosition(path.getCompilationUnit(), path.getLeaf()));
    }
    String symbol() { return "@lambda:" + owner + ":" + offset; }
}
