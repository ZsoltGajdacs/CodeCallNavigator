package codecallnavigator.bytecode;

import com.sun.source.util.JavacTask;
import java.io.IOException;
import java.util.List;
import javax.lang.model.type.TypeKind;
import javax.lang.model.util.Elements;
import javax.tools.JavaCompiler;
import javax.tools.StandardJavaFileManager;

/** Resolves names against compiled output without source declarations shadowing generated types. */
final class CompiledTypes implements AutoCloseable {
    private final StandardJavaFileManager manager;
    private final Elements elements;

    CompiledTypes(JavaCompiler compiler, String classpath) throws IOException {
        manager = compiler.getStandardFileManager(null, null, null);
        try {
            var task = (JavacTask) compiler.getTask(null, manager, null,
                    List.of("-proc:none", "-classpath", classpath, "-sourcepath", ""),
                    List.of("java.lang.Object"), List.of());
            elements = task.getElements();
        } catch (RuntimeException | Error failure) {
            closeAfterFailure(failure);
            throw failure;
        }
    }

    String binaryName(String qualifiedName) throws IOException {
        if (!qualifiedName.contains(".")) {
            throw new IOException("Cannot resolve source declaration type " + qualifiedName + " without a qualified name");
        }
        var type = elements.getTypeElement(qualifiedName);
        if (type == null || type.asType().getKind() == TypeKind.ERROR) {
            throw new IOException("Cannot resolve source declaration type " + qualifiedName + " in compiled output");
        }
        return elements.getBinaryName(type).toString();
    }

    @Override public void close() throws IOException {
        manager.close();
    }

    private void closeAfterFailure(Throwable failure) {
        try {
            manager.close();
        } catch (IOException closeFailure) {
            failure.addSuppressed(closeFailure);
        }
    }
}
