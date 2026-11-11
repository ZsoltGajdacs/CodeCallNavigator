package codecallnavigator.bytecode;

import com.sun.source.util.JavacTask;
import com.sun.source.util.Trees;
import java.util.List;
import javax.lang.model.element.AnnotationMirror;
import javax.lang.model.element.TypeElement;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;

/** Compiler-owned framework identities and generic assignability in one javac context. */
final class SpringTypes {
    static final String PUBLISHER = "org.springframework.context.ApplicationEventPublisher";
    static final String LISTENER = "org.springframework.context.ApplicationListener";
    static final String EVENT = "org.springframework.context.ApplicationEvent";
    static final String PAYLOAD = "org.springframework.context.PayloadApplicationEvent";
    static final String ANNOTATION = "org.springframework.context.event.EventListener";
    static final String TRANSACTION = "org.springframework.transaction.event.TransactionalEventListener";
    static final String ASYNC = "org.springframework.scheduling.annotation.Async";
    final JavacTask task;
    final Trees trees;
    SpringTypes(JavacTask task) { this.task = task; trees = Trees.instance(task); }
    TypeElement external(String name) {
        var element = task.getElements().getTypeElement(name);
        return element != null && trees.getPath(element) == null ? element : null;
    }
    boolean is(TypeMirror type, String name) {
        var expected = external(name);
        return expected != null && type instanceof DeclaredType declared && declared.asElement().equals(expected);
    }
    boolean assignable(TypeMirror from, TypeMirror to) {
        return concrete(from) && concrete(to) && task.getTypes().isAssignable(from, to);
    }
    boolean concrete(TypeMirror type) {
        if (!(type instanceof DeclaredType declared) || type.getKind() == TypeKind.ERROR) return false;
        if (type.toString().equals("java.lang.Object")) return false;
        if (rawHierarchy(declared, new java.util.HashSet<>())) return false;
        return concreteEnclosing(declared) && declared.getTypeArguments().stream().allMatch(this::concrete);
    }
    private boolean rawHierarchy(TypeMirror type, java.util.Set<String> seen) {
        if (!(type instanceof DeclaredType declared)) return false;
        if (!seen.add(type.toString())) return false;
        if (!completeArguments(declared) || !concreteEnclosing(declared)) return true;
        return task.getTypes().directSupertypes(type).stream().anyMatch(parent -> rawHierarchy(parent, seen));
    }
    private boolean completeArguments(DeclaredType type) {
        var declaration = (TypeElement) type.asElement();
        return declaration.getTypeParameters().size() == type.getTypeArguments().size();
    }
    private boolean concreteEnclosing(DeclaredType type) {
        var enclosing = type.getEnclosingType();
        return enclosing.getKind() == TypeKind.NONE || concrete(enclosing);
    }
    DeclaredType implementation(TypeMirror type, String qualified) {
        if (is(type, qualified)) return (DeclaredType) type;
        for (var parent : task.getTypes().directSupertypes(type)) {
            var found = implementation(parent, qualified);
            if (found != null) return found;
        }
        return null;
    }
    List<TypeMirror> events(TypeMirror payload) {
        var event = external(EVENT);
        if (!runtimePayload(payload) || event == null) return List.of();
        if (task.getTypes().isAssignable(payload, event.asType())) return List.of(payload);
        var envelope = external(PAYLOAD);
        return envelope == null ? List.of() : List.of(payload, task.getTypes().getDeclaredType(envelope, payload));
    }
    private boolean runtimePayload(TypeMirror payload) {
        if (!concrete(payload)) return false;
        var declared = (DeclaredType) payload;
        if (declared.getTypeArguments().isEmpty()) return true;
        if (!is(payload, PAYLOAD)) return false;
        var argument = declared.getTypeArguments().getFirst();
        return argument instanceof DeclaredType nested && nested.getTypeArguments().isEmpty();
    }
    AnnotationMirror annotation(javax.lang.model.element.Element element, String name) {
        return element.getAnnotationMirrors().stream().filter(annotation -> is(annotation.getAnnotationType(), name)).findFirst().orElse(null);
    }
}
