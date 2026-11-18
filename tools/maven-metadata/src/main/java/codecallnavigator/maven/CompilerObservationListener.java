package codecallnavigator.maven;

import java.io.IOException;
import java.util.List;
import javax.inject.Named;
import javax.inject.Singleton;
import org.apache.maven.execution.MavenSession;
import org.apache.maven.execution.MojoExecutionEvent;
import org.apache.maven.execution.MojoExecutionListener;
import org.apache.maven.plugin.MojoExecutionException;

/** Observes native compiler mojo boundaries without changing their configuration. */
@Named("codecallnavigator-compiler-observation")
@Singleton
public final class CompilerObservationListener implements MojoExecutionListener {
    private static final String JOURNAL_KEY = "codecallnavigator.compiler-observation.journal.v1";

    @Override
    public void beforeMojoExecution(MojoExecutionEvent event) throws MojoExecutionException {
        observe(event, true);
    }

    @Override
    public void afterMojoExecutionSuccess(MojoExecutionEvent event) throws MojoExecutionException {
        observe(event, false);
    }

    @Override
    public void afterExecutionFailure(MojoExecutionEvent event) {
        if (!compiler(event)) {
            return;
        }
        try {
            CompilerJournal current = journal(event.getSession());
            if (current != null) {
                current.failed(owner(event));
            }
        } catch (IOException failure) {
            if (event.getCause() != null) {
                event.getCause().addSuppressed(failure);
            }
        }
    }

    static void finish(MavenSession session) throws IOException {
        CompilerJournal current = journal(session);
        if (current != null) {
            current.finish();
        }
    }

    private void observe(MojoExecutionEvent event, boolean before) throws MojoExecutionException {
        if (!compiler(event)) {
            return;
        }
        try {
            CompilerJournal current = journal(event.getSession());
            if (current == null) {
                return;
            }
            CompilerJournal.Owner owner = owner(event);
            CompilerJournal.Inputs inputs = MavenCompilerInputs.collect(event);
            if (before) {
                String unsupported = MavenCompilerInputs.unsupported(event, inputs);
                if (unsupported != null) {
                    current.unsupported(owner, inputs, unsupported);
                } else {
                    current.before(owner, inputs);
                }
            } else {
                current.after(owner, inputs, CompilerJournal.Outcome.SUCCESS);
            }
        } catch (IOException | RuntimeException failure) {
            throw new MojoExecutionException("Cannot verify CodeCallNavigator compiler inputs: "
                    + failure.getMessage(), failure);
        }
    }

    private static CompilerJournal journal(MavenSession session) throws IOException {
        String destination = session.getUserProperties().getProperty("codecallnavigator.observation.file");
        if (destination == null || destination.isBlank()) {
            return null;
        }
        // Reactor sessions and both component roles share native repository-session
        // data. No listener injection identity or global mutable registry is needed.
        var data = session.getRepositorySession().getData();
        synchronized (data) {
            CompilerJournal existing = (CompilerJournal) data.get(JOURNAL_KEY);
            if (existing == null) {
                existing = new CompilerJournal(java.nio.file.Path.of(destination),
                        session.getUserProperties().getProperty("codecallnavigator.observation.attempt"));
                data.set(JOURNAL_KEY, existing);
            }
            return existing;
        }
    }

    private static boolean compiler(MojoExecutionEvent event) {
        var execution = event.getExecution();
        return "org.apache.maven.plugins".equals(execution.getGroupId())
                && "maven-compiler-plugin".equals(execution.getArtifactId())
                && List.of("compile", "testCompile").contains(execution.getGoal());
    }

    private static CompilerJournal.Owner owner(MojoExecutionEvent event) {
        String sourceSet = "testCompile".equals(event.getExecution().getGoal()) ? "test" : "main";
        String operation = event.getProject().getId() + ":" + event.getExecution().getGoal()
                + ":" + event.getExecution().getExecutionId();
        return new CompilerJournal.Owner(event.getProject().getBasedir().getAbsolutePath(), sourceSet, operation);
    }

}
