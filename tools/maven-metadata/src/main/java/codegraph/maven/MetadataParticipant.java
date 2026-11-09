package codegraph.maven;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import javax.inject.Inject;
import javax.inject.Named;
import javax.inject.Singleton;
import org.apache.maven.AbstractMavenLifecycleParticipant;
import org.apache.maven.MavenExecutionException;
import org.apache.maven.execution.MavenSession;
import org.apache.maven.rtinfo.RuntimeInformation;

/** Observes the evaluated build; never alters projects, goals or dependency resolution. */
@Named("codegraph-metadata")
@Singleton
public final class MetadataParticipant extends AbstractMavenLifecycleParticipant {
    @Inject
    private RuntimeInformation runtime;

    @Override
    public void afterProjectsRead(MavenSession session) throws MavenExecutionException {
        if (!runtime.isMavenVersion("[3.9,5)")) {
            throw new MavenExecutionException("CodeGraph requires Maven 3.9+ or Maven 4; found "
                    + runtime.getMavenVersion(), new IllegalStateException("Unsupported Maven version"));
        }
        export(session, "codegraph.metadata.initial");
    }

    @Override
    public void afterSessionEnd(MavenSession session) throws MavenExecutionException {
        if (!session.getResult().hasExceptions()) {
            export(session, "codegraph.metadata.final");
        }
    }

    private void export(MavenSession session, String property) throws MavenExecutionException {
        String destination = session.getUserProperties().getProperty(property);
        if (destination == null || destination.isBlank()) {
            return;
        }
        try {
            Path target = Path.of(destination);
            Files.writeString(target, Json.write(BuildMetadata.collect(session, property.endsWith(".final")
                    && session.getRequest().getGoals().contains("test-compile"))), StandardCharsets.UTF_8);
        } catch (IOException | RuntimeException exception) {
            throw new MavenExecutionException("Cannot export CodeGraph Maven metadata", exception);
        }
    }
}
