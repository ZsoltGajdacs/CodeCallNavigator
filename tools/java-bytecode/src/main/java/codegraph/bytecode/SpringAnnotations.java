package codegraph.bytecode;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import javax.lang.model.element.AnnotationMirror;
import javax.lang.model.element.AnnotationValue;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.type.TypeMirror;

/** Qualified direct annotation values; composed custom annotations remain deferred. */
final class SpringAnnotations {
    private final SpringTypes types;
    SpringAnnotations(SpringTypes types) { this.types = types; }
    Map<String, AnnotationValue> values(AnnotationMirror annotation) {
        Map<String, AnnotationValue> result = new LinkedHashMap<>();
        types.task.getElements().getElementValuesWithDefaults(annotation).forEach((key, value) -> result.put(key.getSimpleName().toString(), value));
        return result;
    }
    List<TypeMirror> classes(Map<String, AnnotationValue> values) {
        var first = classes(values.get("value"));
        var second = classes(values.get("classes"));
        if (conflicting(first, second)) return null;
        return first.isEmpty() ? second : first;
    }
    private boolean conflicting(List<TypeMirror> first, List<TypeMirror> second) {
        if (first.isEmpty() || second.isEmpty()) return false;
        return !first.toString().equals(second.toString());
    }
    private List<TypeMirror> classes(AnnotationValue value) {
        var result = new ArrayList<TypeMirror>();
        if (value != null && value.getValue() instanceof List<?> list) {
            for (var item : list) {
                if (item instanceof AnnotationValue entry && entry.getValue() instanceof TypeMirror type) result.add(type);
            }
        }
        return result;
    }
    Map<String, String> qualifiers(ExecutableElement method, AnnotationMirror annotation, boolean transactional) {
        Map<String, String> result = new LinkedHashMap<>();
        var values = values(annotation);
        result.put("spring.condition", value(values, "condition"));
        if (transactional) {
            result.put("spring.transaction_phase", value(values, "phase"));
            result.put("spring.fallback_execution", value(values, "fallbackExecution"));
        } else result.put("spring.default_execution", value(values, "defaultExecution"));
        async(method, result);
        return result;
    }
    void async(ExecutableElement method, Map<String, String> result) {
        boolean async = types.annotation(method, SpringTypes.ASYNC) != null || types.annotation(method.getEnclosingElement(), SpringTypes.ASYNC) != null;
        result.put("spring.async", Boolean.toString(async));
    }
    private String value(Map<String, AnnotationValue> values, String name) {
        var value = values.get(name);
        return value == null ? "" : value.getValue().toString();
    }
}
