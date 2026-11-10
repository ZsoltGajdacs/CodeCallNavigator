package codegraph.bytecode;

import com.sun.source.util.TreePath;
import com.sun.source.util.Trees;
import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.file.Path;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import javax.lang.model.element.AnnotationMirror;
import javax.lang.model.element.Element;
import javax.lang.model.element.ElementKind;
import javax.lang.model.element.TypeElement;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeMirror;

/** Exports attributed declaration facts without imposing architecture or graph policy. */
final class SourceArchitecture {
    private SourceArchitecture() {}

    static void write(List<TreePath> types, List<TreePath> methods, Trees trees, BufferedWriter writer) throws IOException {
        Map<String, Facts> files = new LinkedHashMap<>();
        for (TreePath path : types) {
            if (trees.getElement(path) instanceof TypeElement type) {
                Facts facts = file(files, path);
                facts.names.add(type.getSimpleName().toString());
                facts.names.add(type.getQualifiedName().toString());
                facts.kinds.add(kind(type));
                annotations(type, facts.annotations);
                collectBases(type, facts, new TreeSet<>());
            }
        }
        for (TreePath path : methods) annotations(trees.getElement(path), file(files, path).methodAnnotations);
        for (var entry : files.entrySet()) {
            Facts facts = entry.getValue();
            ContextTsv.write(writer, "architecture", entry.getKey(), String.join(",", facts.namespaces),
                    String.join(",", facts.names), String.join(",", facts.kinds), String.join(",", facts.annotations),
                    String.join(",", facts.methodAnnotations), String.join(",", facts.baseTypes),
                    String.join(",", facts.annotatedInterfaces));
        }
    }

    private static Facts file(Map<String, Facts> files, TreePath path) {
        var unit = path.getCompilationUnit();
        String source = Path.of(unit.getSourceFile().toUri()).toAbsolutePath().normalize().toString();
        Facts facts = files.computeIfAbsent(source, unused -> new Facts());
        if (unit.getPackageName() != null) facts.namespaces.add(unit.getPackageName().toString());
        return facts;
    }

    private static String kind(TypeElement type) {
        return switch (type.getKind()) {
            case ANNOTATION_TYPE -> "annotation";
            case INTERFACE -> "interface";
            case ENUM -> "enum";
            case RECORD -> "record";
            default -> "class";
        };
    }

    private static void annotations(Element element, Collection<String> target) {
        if (element == null) return;
        for (AnnotationMirror annotation : element.getAnnotationMirrors()) {
            if (annotation.getAnnotationType().asElement() instanceof TypeElement type) names(type, target);
        }
    }

    private static void names(TypeElement type, Collection<String> target) {
        target.add(type.getSimpleName().toString());
        target.add(type.getQualifiedName().toString());
    }

    private static void collectBases(TypeElement type, Facts facts, Collection<String> visited) {
        var bases = new java.util.ArrayList<TypeMirror>(type.getInterfaces());
        bases.add(type.getSuperclass());
        for (TypeMirror base : bases) {
            if (!(base instanceof DeclaredType declared) || !(declared.asElement() instanceof TypeElement element)) continue;
            if (!visited.add(element.getQualifiedName().toString())) continue;
            names(element, facts.baseTypes);
            if (element.getKind() == ElementKind.INTERFACE) annotations(element, facts.annotatedInterfaces);
            collectBases(element, facts, visited);
        }
    }

    private static final class Facts {
        final TreeSet<String> namespaces = new TreeSet<>();
        final TreeSet<String> names = new TreeSet<>();
        final TreeSet<String> kinds = new TreeSet<>();
        final TreeSet<String> annotations = new TreeSet<>();
        final TreeSet<String> methodAnnotations = new TreeSet<>();
        final TreeSet<String> baseTypes = new TreeSet<>();
        final TreeSet<String> annotatedInterfaces = new TreeSet<>();
    }
}
