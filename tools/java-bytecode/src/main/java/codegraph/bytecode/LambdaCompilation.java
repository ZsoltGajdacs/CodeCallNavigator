package codegraph.bytecode;

import com.sun.source.util.JavacTask;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.net.URI;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import javax.tools.*;
import org.apache.bcel.classfile.ClassParser;
import org.apache.bcel.classfile.JavaClass;

/** Optional processor-free replay. Compiler outputs exist only in memory. */
final class LambdaCompilation {
    record Context(Set<Path> sources, String classpath, CompiledTypes compiled) {}
    record Result(Map<String, JavaClass> classes, Map<LambdaSourceSite, String> implementations, String failure) {}
    private LambdaCompilation() {}

    static Result run(Context context, int release, String debug) {
        var compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (var standard = compiler.getStandardFileManager(diagnostics, null, null);
                var manager = new Outputs(standard)) {
            var options = List.of("-proc:none", "-implicit:none", "-classpath", context.classpath(),
                    "--release", Integer.toString(release), debug);
            var task = (JavacTask) compiler.getTask(null, manager, diagnostics, options, null,
                    standard.getJavaFileObjectsFromPaths(context.sources()));
            var lowering = new LambdaLowering(task, context.compiled());
            task.addTaskListener(lowering);
            task.parse();
            task.analyze();
            if (hasErrors(diagnostics)) return failed("Source attribution prevents compiler replay");
            task.generate();
            if (hasErrors(diagnostics)) return failed("Compiler replay did not generate complete output");
            if (!lowering.failure().isEmpty()) return failed(lowering.failure());
            return new Result(manager.classes(), lowering.implementations(), "");
        } catch (IOException | RuntimeException | LinkageError error) {
            return failed("Compiler replay unavailable: " + error.getClass().getSimpleName());
        }
    }

    private static boolean hasErrors(DiagnosticCollector<JavaFileObject> diagnostics) {
        return diagnostics.getDiagnostics().stream().anyMatch(diagnostic -> diagnostic.getKind() == Diagnostic.Kind.ERROR);
    }
    private static Result failed(String reason) { return new Result(Map.of(), Map.of(), reason); }

    private static final class Outputs extends ForwardingJavaFileManager<StandardJavaFileManager> {
        private final Map<String, ByteArrayOutputStream> outputs = new HashMap<>();
        Outputs(StandardJavaFileManager manager) { super(manager); }
        @Override public JavaFileObject getJavaFileForOutput(Location location, String name,
                JavaFileObject.Kind kind, FileObject sibling) throws IOException {
            if (kind != JavaFileObject.Kind.CLASS) throw new IOException("Unexpected compiler replay output");
            var output = new ByteArrayOutputStream();
            if (outputs.putIfAbsent(name, output) != null) throw new IOException("Duplicate compiler replay class");
            return new SimpleJavaFileObject(URI.create("memory:///" + name.replace('.', '/') + kind.extension), kind) {
                @Override public java.io.OutputStream openOutputStream() { return output; }
            };
        }
        Map<String, JavaClass> classes() throws IOException {
            var result = new HashMap<String, JavaClass>();
            for (var entry : outputs.entrySet()) {
                result.put(entry.getKey(), new ClassParser(new java.io.ByteArrayInputStream(entry.getValue().toByteArray()), entry.getKey()).parse());
            }
            return result;
        }
    }
}
