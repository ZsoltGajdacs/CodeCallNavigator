package codegraph.bytecode;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import javax.lang.model.element.AnnotationMirror;
import javax.lang.model.element.AnnotationValue;
import javax.lang.model.element.Element;
import javax.lang.model.element.TypeElement;

/** Compiler-qualified mapping annotations; unknown composition is retained as a blocker. */
final class SpringHttpAnnotations {
    static final String REQUEST = "org.springframework.web.bind.annotation.RequestMapping";
    private static final String PREFIX = "org.springframework.web.bind.annotation.";
    private static final Map<String, String> SHORTCUTS = Map.of(
            "GetMapping", "GET", "PostMapping", "POST", "PutMapping", "PUT",
            "PatchMapping", "PATCH", "DeleteMapping", "DELETE");
    private final SpringTypes types;
    private final SpringAnnotations annotations;
    record Mapping(List<String> paths, List<String> methods, List<String> restrictions) {}
    SpringHttpAnnotations(SpringTypes types) { this.types = types; annotations = new SpringAnnotations(types); }
    boolean controller(TypeElement type) {
        return type.getAnnotationMirrors().stream().anyMatch(annotation -> controller(annotation, new java.util.HashSet<>()));
    }
    private boolean controller(AnnotationMirror annotation, Set<String> seen) {
        if (types.is(annotation.getAnnotationType(), "org.springframework.stereotype.Controller")) return true;
        var element = (TypeElement) annotation.getAnnotationType().asElement();
        if (!seen.add(element.getQualifiedName().toString())) return false;
        return element.getAnnotationMirrors().stream().anyMatch(parent -> controller(parent, seen));
    }
    List<Mapping> mappings(Element element) {
        return element.getAnnotationMirrors().stream().filter(annotation -> mapping(annotation, new java.util.HashSet<>())).map(this::read).toList();
    }
    private boolean mapping(AnnotationMirror annotation, Set<String> seen) {
        if (types.is(annotation.getAnnotationType(), REQUEST)) return true;
        var element = (TypeElement) annotation.getAnnotationType().asElement();
        if (!seen.add(element.getQualifiedName().toString())) return false;
        return element.getAnnotationMirrors().stream().anyMatch(parent -> mapping(parent, seen));
    }
    private Mapping read(AnnotationMirror annotation) {
        var name = ((TypeElement) annotation.getAnnotationType().asElement()).getQualifiedName().toString();
        var values = annotations.values(annotation);
        var shortcut = SHORTCUTS.entrySet().stream().filter(entry -> types.is(annotation.getAnnotationType(), PREFIX + entry.getKey())).map(Map.Entry::getValue).findFirst();
        if (!types.is(annotation.getAnnotationType(), REQUEST) && shortcut.isEmpty()) {
            return new Mapping(java.util.Collections.singletonList(null), List.of(), List.of("spring.composed_mapping:" + name));
        }
        var restrictions = restrictions(values);
        var paths = paths(values, restrictions);
        var methods = new TreeSet<>(strings(values.get("method")));
        shortcut.ifPresent(methods::add);
        if (methods.contains("GET")) methods.add("HEAD");
        return new Mapping(paths, new ArrayList<>(methods), restrictions);
    }
    private List<String> paths(Map<String, AnnotationValue> values, List<String> restrictions) {
        var value = strings(values.get("value"));
        var path = strings(values.get("path"));
        if (conflicting(value, path)) {
            restrictions.add("spring.conflicting_path_aliases");
            return java.util.Collections.singletonList(null);
        }
        var selected = value.isEmpty() ? path : value;
        return selected.isEmpty() ? List.of("") : selected;
    }
    private boolean conflicting(List<String> value, List<String> path) {
        if (value.isEmpty() || path.isEmpty()) return false;
        return !value.equals(path);
    }
    private List<String> restrictions(Map<String, AnnotationValue> values) {
        var result = new ArrayList<String>();
        for (var name : List.of("params", "headers", "consumes", "produces")) {
            for (var value : strings(values.get(name))) result.add("spring." + name + ":" + value);
        }
        return result;
    }
    private List<String> strings(AnnotationValue value) {
        if (value == null || !(value.getValue() instanceof List<?> list)) return List.of();
        return list.stream().map(item -> ((AnnotationValue) item).getValue().toString()).toList();
    }
}
