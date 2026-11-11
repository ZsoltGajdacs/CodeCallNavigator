package codegraph.bytecode;

import com.sun.source.tree.ClassTree;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import javax.lang.model.element.TypeElement;

/** Lexical type declarations only: erroneous expressions can also resolve to TypeElements. */
final class LambdaOwner {
    private LambdaOwner() {}

    static TypeElement declaredAt(TreePath path, Trees trees) {
        if (path == null || !(path.getLeaf() instanceof ClassTree)) return null;
        return trees.getElement(path) instanceof TypeElement type ? type : null;
    }

    static TypeElement enclosing(TreePath path, Trees trees) {
        for (var parent = path.getParentPath(); parent != null; parent = parent.getParentPath()) {
            var type = declaredAt(parent, trees);
            if (type != null) return type;
        }
        return null;
    }
}
