package codecallnavigator.bytecode;

import com.sun.source.util.JavacTask;
import java.nio.file.Path;
import java.util.List;
import java.util.Set;
import javax.tools.Diagnostic;
import javax.tools.DiagnosticCollector;
import javax.tools.JavaFileObject;
import javax.tools.ToolProvider;

/** A real compiler module-resolution failure has no source unit. */
public final class AttributionProbe {
    public static void main(String[] args) throws Exception {
        var source = Path.of(args[0]);
        var compiler = ToolProvider.getSystemJavaCompiler();
        var diagnostics = new DiagnosticCollector<JavaFileObject>();
        try (var files = compiler.getStandardFileManager(diagnostics, null, null)) {
            var task = (JavacTask) compiler.getTask(null, files, diagnostics,
                    List.of("--add-modules", "ccn.missing.module"), null, files.getJavaFileObjects(source));
            var unit = task.parse().iterator().next();
            task.analyze();
            boolean unowned = diagnostics.getDiagnostics().stream()
                    .anyMatch(diagnostic -> diagnostic.getKind() == Diagnostic.Kind.ERROR && diagnostic.getSource() == null);
            if (!unowned) throw new AssertionError("Expected an unowned compiler error");
            var attribution = new FlowAttribution(Set.of(source), diagnostics.getDiagnostics());
            if (attribution.complete(unit)) throw new AssertionError("Unowned errors must leave all facts unknown");
            System.out.println("UNOWNED_COMPILER_ERROR_REMAINS_UNKNOWN");
        }
    }
}
