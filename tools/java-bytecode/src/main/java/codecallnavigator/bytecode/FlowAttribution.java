package codecallnavigator.bytecode;

import com.sun.source.tree.CompilationUnitTree;
import java.net.URI;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import javax.tools.Diagnostic;
import javax.tools.JavaFileObject;

/** Error recovery can return a method symbol; only clean owned units prove new facts. */
final class FlowAttribution {
    static final FlowAttribution UNKNOWN = new FlowAttribution();
    private final Set<URI> errors;
    private final Set<URI> owned;
    private final boolean unownedError;

    private FlowAttribution() { errors = Set.of(); owned = Set.of(); unownedError = true; }

    FlowAttribution(Set<Path> sources, List<? extends Diagnostic<?>> diagnostics) {
        var owned = new HashSet<URI>();
        for (Path source : sources) owned.add(source.toUri().normalize());
        var failed = new HashSet<URI>();
        boolean unowned = false;
        for (var diagnostic : diagnostics) {
            if (diagnostic.getKind() != Diagnostic.Kind.ERROR) continue;
            URI source = source(diagnostic);
            if (owned.contains(source)) failed.add(source);
            else unowned = true;
        }
        this.owned = Set.copyOf(owned);
        errors = Set.copyOf(failed);
        unownedError = unowned;
    }

    boolean complete(CompilationUnitTree unit) {
        var file = unit.getSourceFile().toUri().normalize();
        return !unownedError && owned.contains(file) && !errors.contains(file);
    }

    private static URI source(Diagnostic<?> diagnostic) {
        return diagnostic.getSource() instanceof JavaFileObject file ? file.toUri().normalize() : null;
    }
}
