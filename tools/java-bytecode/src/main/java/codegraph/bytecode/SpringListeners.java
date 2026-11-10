package codegraph.bytecode;

import com.sun.source.tree.MethodTree;
import com.sun.source.util.TreePath;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.type.TypeMirror;

/** Source-owned listener candidates; compiler types perform every payload match. */
final class SpringListeners {
    record Listener(TreePath path, ExecutableElement method, List<TypeMirror> accepted, Map<String, String> qualifiers) {
        String semantics() {
            if (qualifiers.get("spring.async").equals("true")) return "async-boundary";
            return qualifiers.containsKey("spring.transaction_phase") ? "deferred" : "possible-invocation";
        }
    }
    private final SpringTypes types;
    private final SpringAnnotations annotations;
    SpringListeners(SpringTypes types) { this.types = types; annotations = new SpringAnnotations(types); }
    List<Listener> collect(List<TreePath> methods) {
        return methods.stream().map(this::listener).filter(java.util.Objects::nonNull).toList();
    }
    private Listener listener(TreePath path) {
        if (((MethodTree) path.getLeaf()).getBody() == null) return null;
        if (!(types.trees.getElement(path) instanceof ExecutableElement method)) return null;
        return java.util.Optional.ofNullable(annotated(path, method)).orElseGet(() -> implemented(path, method));
    }
    private Listener annotated(TreePath path, ExecutableElement method) {
        var annotation = types.annotation(method, SpringTypes.TRANSACTION);
        boolean transactional = annotation != null;
        if (annotation == null) annotation = types.annotation(method, SpringTypes.ANNOTATION);
        if (annotation == null || method.getParameters().size() > 1) return null;
        var accepted = accepted(method, annotation);
        if (accepted == null || accepted.isEmpty()) return null;
        return new Listener(path, method, accepted, annotations.qualifiers(method, annotation, transactional));
    }
    private List<TypeMirror> accepted(ExecutableElement method, javax.lang.model.element.AnnotationMirror annotation) {
        var accepted = annotations.classes(annotations.values(annotation));
        if (accepted == null) return null;
        if (accepted.isEmpty()) return parameter(method);
        if (accepted.stream().anyMatch(type -> !types.concrete(type))) return null;
        if (!fitsParameter(method, accepted)) return null;
        return accepted;
    }
    private List<TypeMirror> parameter(ExecutableElement method) {
        if (method.getParameters().size() != 1) return null;
        var parameter = method.getParameters().getFirst().asType();
        return types.concrete(parameter) ? List.of(parameter) : null;
    }
    private boolean fitsParameter(ExecutableElement method, List<TypeMirror> accepted) {
        if (method.getParameters().isEmpty()) return true;
        var parameter = method.getParameters().getFirst().asType();
        return accepted.stream().allMatch(type -> types.assignable(type, parameter));
    }
    private Listener implemented(TreePath path, ExecutableElement method) {
        if (!method.getSimpleName().contentEquals("onApplicationEvent") || method.getParameters().size() != 1) return null;
        var contract = types.implementation(method.getEnclosingElement().asType(), SpringTypes.LISTENER);
        if (contract == null || contract.getTypeArguments().size() != 1) return null;
        var expected = contract.getTypeArguments().getFirst();
        if (!types.concrete(expected) || !types.task.getTypes().isSameType(expected, method.getParameters().getFirst().asType())) return null;
        Map<String, String> qualifiers = new LinkedHashMap<>();
        annotations.async(method, qualifiers);
        return new Listener(path, method, List.of(expected), qualifiers);
    }
}
