package codegraph.bytecode;

import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.util.JavacTask;
import com.sun.source.util.TreePath;
import java.io.BufferedWriter;
import java.io.IOException;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.Modifier;
import javax.lang.model.element.TypeElement;

/** Source-context HTTP inventory; unsupported declarations survive as blockers. */
final class SourceHttp {
    private record Handler(TreePath path, ExecutableElement method) {}
    private record Route(TreePath source, String symbol, String path, List<String> methods, List<String> restrictions) {}
    private final SpringTypes types;
    private final SpringHttpAnnotations annotations;
    private final CompiledTypes compiled;
    private final BufferedWriter writer;
    private final Map<CompilationUnitTree, FlowSource> sources = new IdentityHashMap<>();
    SourceHttp(JavacTask task, CompiledTypes compiled, BufferedWriter writer) {
        types = new SpringTypes(task); annotations = new SpringHttpAnnotations(types);
        this.compiled = compiled; this.writer = writer;
    }
    void write(List<TreePath> declarations, List<TreePath> methods) throws IOException {
        if (types.external(SpringHttpAnnotations.REQUEST) == null) return;
        for (var path : declarations) {
            if (!(types.trees.getElement(path) instanceof TypeElement type)) continue;
            if (!controller(type, new java.util.HashSet<>()) || type.getModifiers().contains(Modifier.ABSTRACT)) continue;
            var prefixes = annotations.mappings(type);
            if (prefixes.isEmpty()) prefixes = List.of(new SpringHttpAnnotations.Mapping(List.of(""), List.of(), List.of()));
            for (var method : methods) own(type, method, prefixes);
            inherited(path, type, new java.util.HashSet<>());
        }
    }
    private boolean controller(TypeElement type, java.util.Set<String> seen) {
        if (!seen.add(type.getQualifiedName().toString())) return false;
        if (annotations.controller(type)) return true;
        return types.task.getTypes().directSupertypes(type.asType()).stream()
                .map(types.task.getTypes()::asElement).filter(TypeElement.class::isInstance)
                .map(TypeElement.class::cast).anyMatch(parent -> controller(parent, seen));
    }
    private void own(TypeElement type, TreePath path, List<SpringHttpAnnotations.Mapping> prefixes) throws IOException {
        if (!(types.trees.getElement(path) instanceof ExecutableElement method)) return;
        if (!method.getEnclosingElement().equals(type)) return;
        var mappings = annotations.mappings(method);
        for (var prefix : prefixes) for (var mapping : mappings) {
            var restrictions = new ArrayList<String>();
            if (prefixes.size() > 1 || mappings.size() > 1) restrictions.add("spring.multiple_mapping_annotations");
            emit(new Handler(path, method), prefix, mapping, restrictions);
        }
    }
    private void emit(Handler handler, SpringHttpAnnotations.Mapping prefix,
            SpringHttpAnnotations.Mapping mapping, List<String> extra) throws IOException {
        for (var left : prefix.paths()) for (var right : mapping.paths()) {
            var restrictions = new ArrayList<>(extra);
            restrictions.addAll(prefix.restrictions()); restrictions.addAll(mapping.restrictions());
            var combined = SpringHttpPaths.combine(left, right, restrictions);
            var methods = new java.util.TreeSet<>(prefix.methods()); methods.addAll(mapping.methods());
            route(new Route(handler.path(), SourceSignatures.method(handler.method(), types.task, compiled).key(), combined, new ArrayList<>(methods), restrictions));
        }
    }
    private void inherited(TreePath owner, TypeElement type, java.util.Set<String> seen) throws IOException {
        for (var parentType : types.task.getTypes().directSupertypes(type.asType())) {
            if (!(types.task.getTypes().asElement(parentType) instanceof TypeElement parent)) continue;
            if (!seen.add(parent.getQualifiedName().toString())) continue;
            if (!annotations.mappings(parent).isEmpty()) blocker(owner, "spring.inherited_type_mapping:" + parent.getQualifiedName());
            inheritedMethods(owner, parent);
            inherited(owner, parent, seen);
        }
    }
    private void inheritedMethods(TreePath owner, TypeElement parent) throws IOException {
        for (var element : parent.getEnclosedElements()) {
            if (element instanceof ExecutableElement method && !annotations.mappings(method).isEmpty()) {
                blocker(owner, "spring.inherited_method_mapping:" + parent.getQualifiedName() + ":" + method);
            }
        }
    }
    private void blocker(TreePath owner, String reason) throws IOException {
        route(new Route(owner, null, null, List.of(), List.of(reason)));
    }
    private FlowSource source(TreePath path) throws IOException {
        var unit = path.getCompilationUnit();
        if (!sources.containsKey(unit)) sources.put(unit, new FlowSource(unit, types.task, compiled));
        return sources.get(unit);
    }
    private void route(Route route) throws IOException {
        var source = source(route.source());
        var tree = route.source().getLeaf();
        long start = source.start(tree);
        var site = FlowJson.object("file", source.file, "line", source.unit.getLineMap().getLineNumber(start),
                "column", source.unit.getLineMap().getColumnNumber(start), "span", source.span(tree));
        var fact = FlowJson.object("symbol", route.symbol(), "path", route.path(), "methods", route.methods(),
                "site", site, "restrictions", route.restrictions());
        ContextTsv.write(writer, "http-route", "", "", FlowJson.encode(fact), "", "", "", "", "");
    }
}
