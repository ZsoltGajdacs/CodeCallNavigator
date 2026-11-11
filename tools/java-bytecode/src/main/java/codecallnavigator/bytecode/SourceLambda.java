package codecallnavigator.bytecode;

import com.sun.source.tree.AssignmentTree;
import com.sun.source.tree.BlockTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.tree.ParenthesizedTree;
import com.sun.source.tree.TypeCastTree;
import com.sun.source.tree.VariableTree;
import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.io.BufferedWriter;
import java.io.IOException;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.TypeElement;
import javax.lang.model.element.VariableElement;
import javax.lang.model.util.Elements;

/** Source names and positions only; bytecode-to-source matching belongs to normalization. */
final class SourceLambda {
    private SourceLambda() {}

    static void write(TreePath path, Trees trees, Elements elements, BufferedWriter writer) throws IOException {
        var owner = declaringType(path, trees);
        var unit = path.getCompilationUnit();
        var positions = trees.getSourcePositions();
        long start = positions.getStartPosition(unit, path.getLeaf());
        long end = positions.getEndPosition(unit, path.getLeaf());
        if (start < 0 || end <= start) throw new IOException("Cannot locate source lambda in " + owner);
        var lines = unit.getLineMap();
        ContextTsv.write(writer, "lambda", elements.getBinaryName(owner), "",
                lines.getLineNumber(start), lines.getLineNumber(end - 1),
                lines.getColumnNumber(start), lines.getColumnNumber(end - 1),
                context(path, trees), assignedName(path, trees));
    }

    private static TypeElement declaringType(TreePath path, Trees trees) throws IOException {
        var owner = LambdaOwner.enclosing(path, trees);
        if (owner == null) throw new IOException("Cannot identify lambda's declaring type");
        return owner;
    }

    private static String context(TreePath path, Trees trees) {
        for (var enclosing = path.getParentPath(); enclosing != null; enclosing = enclosing.getParentPath()) {
            if (enclosing.getLeaf() instanceof MethodTree method) {
                return methodContext(method);
            }
            if (isTypeMember(enclosing, trees)) return initializerContext(enclosing);
            if (LambdaOwner.declaredAt(enclosing, trees) != null) break;
        }
        return "Instance initializer";
    }

    private static String methodContext(MethodTree method) {
        return method.getName().contentEquals("<init>") ? "Constructor" : method.getName().toString();
    }

    private static boolean isTypeMember(TreePath path, Trees trees) {
        var parent = path.getParentPath();
        return LambdaOwner.declaredAt(parent, trees) != null
                && (path.getLeaf() instanceof VariableTree || path.getLeaf() instanceof BlockTree);
    }

    private static String initializerContext(TreePath path) {
        boolean isStatic = path.getLeaf() instanceof BlockTree block ? block.isStatic()
                : ((VariableTree) path.getLeaf()).getModifiers().getFlags().contains(Modifier.STATIC);
        return isStatic ? "Static initializer" : "Instance initializer";
    }

    private static String assignedName(TreePath path, Trees trees) {
        var expression = unwrap(path);
        var parent = expression.getParentPath();
        if (parent == null) return "";
        if (parent.getLeaf() instanceof VariableTree variable && variable.getInitializer() == expression.getLeaf()) {
            return variable.getName().toString();
        }
        return assignmentName(expression, parent, trees);
    }

    private static String assignmentName(TreePath expression, TreePath parent, Trees trees) {
        if (parent.getLeaf() instanceof AssignmentTree assignment && assignment.getExpression() == expression.getLeaf()) {
            var target = trees.getElement(new TreePath(parent, assignment.getVariable()));
            if (target instanceof VariableElement variable) return variable.getSimpleName().toString();
        }
        return "";
    }

    private static TreePath unwrap(TreePath path) {
        var expression = path;
        for (var parent = expression.getParentPath(); parent != null; parent = expression.getParentPath()) {
            if (!(parent.getLeaf() instanceof ParenthesizedTree || parent.getLeaf() instanceof TypeCastTree)) break;
            expression = parent;
        }
        return expression;
    }
}
