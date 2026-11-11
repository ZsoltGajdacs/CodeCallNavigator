package codecallnavigator.bytecode;


import com.sun.source.tree.AssignmentTree;
import com.sun.source.tree.MethodInvocationTree;
import com.sun.source.tree.NewClassTree;
import com.sun.source.tree.ParenthesizedTree;
import com.sun.source.tree.TypeCastTree;
import com.sun.source.tree.VariableTree;
import com.sun.source.util.TreePath;

/** Syntactic roles contain compiler names, never source positions or literal values. */
final class LambdaSourceRole {
    private LambdaSourceRole() {}
    static String role(TreePath path) {
        var expression = unwrap(path);
        var parent = expression.getParentPath();
        if (parent == null) return "root";
        var tree = parent.getLeaf();
        if (tree instanceof VariableTree variable) return "initializer:" + variable.getName();
        if (tree instanceof AssignmentTree) return "assignment";
        if (tree instanceof MethodInvocationTree call) return "invocation-argument:" + call.getArguments().indexOf(expression.getLeaf());
        if (tree instanceof NewClassTree call) return "constructor-argument:" + call.getArguments().indexOf(expression.getLeaf());
        return tree.getKind().toString();
    }
    private static TreePath unwrap(TreePath path) {
        for (var parent = path.getParentPath(); parent != null; parent = path.getParentPath()) {
            if (!(parent.getLeaf() instanceof ParenthesizedTree || parent.getLeaf() instanceof TypeCastTree)) break;
            path = parent;
        }
        return path;
    }
}
