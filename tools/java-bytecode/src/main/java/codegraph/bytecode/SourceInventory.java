package codegraph.bytecode;

import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.tree.ClassTree;
import com.sun.source.tree.LambdaExpressionTree;
import com.sun.source.tree.MethodTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import com.sun.source.util.TreePathScanner;
import com.sun.source.util.Trees;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.TreeSet;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.TypeElement;
import javax.lang.model.type.ArrayType;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;
import javax.lang.model.util.Elements;
import javax.lang.model.util.Types;
import javax.tools.ToolProvider;

/** Discovers source declarations, independently of bytecode and graph inclusion policy. */
public final class SourceInventory {
    private SourceInventory() {}

    public static void main(String[] args) throws IOException {
        if (args.length < 3) throw new IllegalArgumentException("Usage: SourceInventory OUTPUT CLASSPATH CLASS_DIRS SOURCE_ROOT...");
        var sources = CompiledSources.collect(args[2], java.util.Arrays.copyOfRange(args, 3, args.length));
        try (var writer = Files.newBufferedWriter(Path.of(args[0]))) {
            ContextTsv.write(writer, "kind_v4", "signature", "return_type", "start_line", "end_line",
                    "start_column", "end_column", "context", "assigned_name");
            if (sources.isEmpty()) return;
            inventory(sources, args[1], writer);
        }
    }

    private static void inventory(TreeSet<Path> sources, String classpath, java.io.BufferedWriter writer) throws IOException {
        var compiler = ToolProvider.getSystemJavaCompiler();
        if (compiler == null) throw new IOException("Source inventory requires a full JDK");
        try (var manager = compiler.getStandardFileManager(null, null, null);
                var compiled = new CompiledTypes(compiler, classpath)) {
            var diagnostics = new javax.tools.DiagnosticCollector<javax.tools.JavaFileObject>();
            var task = (JavacTask) compiler.getTask(null, manager, diagnostics,
                    List.of("-proc:none", "-implicit:none", "-classpath", classpath), null,
                    manager.getJavaFileObjectsFromPaths(sources));
            List<TreePath> declarations = new ArrayList<>();
            List<TreePath> lambdas = new ArrayList<>();
            List<TreePath> types = new ArrayList<>();
            collectTrees(task, declarations, lambdas, types);
            var parseError = diagnostics.getDiagnostics().stream()
                    .filter(diagnostic -> diagnostic.getKind() == javax.tools.Diagnostic.Kind.ERROR).findFirst();
            if (parseError.isPresent()) throw new IOException("Cannot parse project source: " + parseError.get());
            task.analyze();
            var trees = Trees.instance(task);
            writeMethods(declarations, task, compiled, writer);
            SourceArchitecture.write(types, declarations, trees, writer);
            new SourceFlow(task, compiled, writer).write(declarations, lambdas);
            for (TreePath path : lambdas) SourceLambda.write(path, trees, task.getElements(), writer);
        }
    }

    private static void collectTrees(JavacTask task, List<TreePath> declarations, List<TreePath> lambdas, List<TreePath> types) throws IOException {
        // Snapshot parsed trees before attribution can add implicit declarations.
        for (CompilationUnitTree unit : task.parse()) {
            new TreePathScanner<Void, Void>() {
                @Override public Void visitClass(ClassTree tree, Void unused) {
                    types.add(getCurrentPath());
                    return super.visitClass(tree, unused);
                }
                @Override public Void visitMethod(MethodTree tree, Void unused) {
                    declarations.add(getCurrentPath());
                    return super.visitMethod(tree, unused);
                }
                @Override public Void visitLambdaExpression(LambdaExpressionTree tree, Void unused) {
                    lambdas.add(getCurrentPath());
                    return super.visitLambdaExpression(tree, unused);
                }
            }.scan(unit, null);
        }
    }

    private static void writeMethods(List<TreePath> declarations, JavacTask task, CompiledTypes compiled,
            java.io.BufferedWriter writer) throws IOException {
        var trees = Trees.instance(task);
        for (TreePath path : declarations) {
            if (!(trees.getElement(path) instanceof ExecutableElement method)) {
                throw new IOException("Cannot attribute source declaration " + path.getLeaf());
            }
            writeMethod(method, task, compiled, writer);
        }
    }

    private static void writeMethod(ExecutableElement method, JavacTask task, CompiledTypes compiled,
            java.io.BufferedWriter writer) throws IOException {
        var types = task.getTypes();
        var elements = task.getElements();
        String owner = elements.getBinaryName((TypeElement) method.getEnclosingElement()).toString();
        try {
            var signature = SourceSignatures.method(method, task, compiled);
            ContextTsv.write(writer, "method", signature.signature(), signature.returns(), 0, 0, 0, 0, "", "");
        } catch (IOException error) {
            throw new IOException("Cannot inventory " + owner + ":" + method + ": " + error.getMessage(), error);
        }
    }

}
