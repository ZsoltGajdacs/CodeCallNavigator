package codegraph.bytecode;

import com.sun.source.tree.CompilationUnitTree;
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
            ContextTsv.write(writer, "kind", "signature", "return_type", "start_line", "end_line");
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
            collectTrees(task, declarations, lambdas);
            var parseError = diagnostics.getDiagnostics().stream()
                    .filter(diagnostic -> diagnostic.getKind() == javax.tools.Diagnostic.Kind.ERROR).findFirst();
            if (parseError.isPresent()) throw new IOException("Cannot parse project source: " + parseError.get());
            task.analyze();
            var trees = Trees.instance(task);
            writeMethods(declarations, task, compiled, writer);
            for (TreePath path : lambdas) writeLambda(path, trees, task.getElements(), writer);
        }
    }

    private static void collectTrees(JavacTask task, List<TreePath> declarations, List<TreePath> lambdas) throws IOException {
        // Snapshot parsed trees before attribution can add implicit declarations.
        for (CompilationUnitTree unit : task.parse()) {
            new TreePathScanner<Void, Void>() {
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
            var arguments = new ArrayList<String>();
            for (var parameter : method.getParameters()) arguments.add(typeName(parameter.asType(), types, elements, compiled));
            ContextTsv.write(writer, "method", owner + ":" + method.getSimpleName() + "(" + String.join(",", arguments) + ")",
                    typeName(method.getReturnType(), types, elements, compiled), 0, 0);
        } catch (IOException error) {
            throw new IOException("Cannot inventory " + owner + ":" + method + ": " + error.getMessage(), error);
        }
    }

    private static void writeLambda(TreePath path, Trees trees, Elements elements, java.io.BufferedWriter writer) throws IOException {
        TreePath enclosing = path.getParentPath();
        while (enclosing != null && !(trees.getElement(enclosing) instanceof TypeElement)) enclosing = enclosing.getParentPath();
        if (enclosing == null) throw new IOException("Cannot identify lambda's declaring type");
        String owner = elements.getBinaryName((TypeElement) trees.getElement(enclosing)).toString();
        var unit = path.getCompilationUnit();
        var positions = trees.getSourcePositions();
        long start = positions.getStartPosition(unit, path.getLeaf());
        long end = positions.getEndPosition(unit, path.getLeaf());
        ContextTsv.write(writer, "lambda", owner, "", unit.getLineMap().getLineNumber(start),
                unit.getLineMap().getLineNumber(Math.max(start, end - 1)));
    }

    private static String typeName(TypeMirror original, Types types, Elements elements, CompiledTypes compiled) throws IOException {
        TypeMirror type = types.erasure(original);
        if (type.getKind() == TypeKind.ERROR) return compiled.binaryName(errorTypeName(type));
        if (type instanceof ArrayType array) return typeName(array.getComponentType(), types, elements, compiled) + "[]";
        if (type instanceof DeclaredType declared) return elements.getBinaryName((TypeElement) declared.asElement()).toString();
        return type.toString();
    }

    private static String errorTypeName(TypeMirror type) {
        if (type instanceof DeclaredType declared && declared.asElement() instanceof TypeElement element) {
            return element.getQualifiedName().toString();
        }
        return type.toString();
    }
}
