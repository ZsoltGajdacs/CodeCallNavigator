package codegraph.bytecode;

import com.sun.source.util.JavacTask;
import java.io.IOException;
import java.util.ArrayList;
import javax.lang.model.element.ExecutableElement;
import javax.lang.model.element.TypeElement;
import javax.lang.model.type.ArrayType;
import javax.lang.model.type.DeclaredType;
import javax.lang.model.type.TypeKind;
import javax.lang.model.type.TypeMirror;

/** Existing erased compiler identity shared by declaration inventory and flow attribution. */
final class SourceSignatures {
    private SourceSignatures() {}
    record Method(String signature, String returns) { String key() { return signature + "|" + returns; } }

    static Method method(ExecutableElement method, JavacTask task, CompiledTypes compiled) throws IOException {
        String owner = task.getElements().getBinaryName((TypeElement) method.getEnclosingElement()).toString();
        var arguments = new ArrayList<String>();
        for (var parameter : method.getParameters()) arguments.add(type(parameter.asType(), task, compiled));
        return new Method(owner + ":" + method.getSimpleName() + "(" + String.join(",", arguments) + ")",
                type(method.getReturnType(), task, compiled));
    }

    private static String type(TypeMirror original, JavacTask task, CompiledTypes compiled) throws IOException {
        TypeMirror type = task.getTypes().erasure(original);
        if (type.getKind() == TypeKind.ERROR) return compiled.binaryName(errorTypeName(type));
        if (type instanceof ArrayType array) return type(array.getComponentType(), task, compiled) + "[]";
        if (type instanceof DeclaredType declared) return task.getElements().getBinaryName((TypeElement) declared.asElement()).toString();
        return type.toString();
    }

    private static String errorTypeName(TypeMirror type) {
        if (type instanceof DeclaredType declared && declared.asElement() instanceof TypeElement element) {
            return element.getQualifiedName().toString();
        }
        return type.toString();
    }
}
