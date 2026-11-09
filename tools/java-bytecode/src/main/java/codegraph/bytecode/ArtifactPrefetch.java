package codegraph.bytecode;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;

/** Bounded artifact preparation; selection consumes the original classpath order. */
final class ArtifactPrefetch implements AutoCloseable {
    record Input(Path original, boolean project) {}
    record Prepared(Input input, List<ArtifactInventory.Definition> definitions) {}

    @FunctionalInterface
    interface Visitor {
        void accept(Prepared prepared) throws IOException;
    }

    private final ArtifactInventory inventory;
    private final ExecutorService executor = Executors.newFixedThreadPool(2);
    private final Iterator<Input> remaining;
    private final Map<Path, Boolean> retainProjectBytes = new HashMap<>();
    private final ArrayDeque<Future<Prepared>> pending = new ArrayDeque<>();

    ArtifactPrefetch(ArtifactInventory inventory, List<Input> inputs) {
        this.inventory = inventory;
        remaining = inputs.iterator();
        for (Input input : inputs) retainProjectBytes.merge(input.original(), input.project(), Boolean::logicalOr);
    }

    void visit(Visitor visitor) throws IOException {
        enqueue();
        enqueue();
        while (!pending.isEmpty()) {
            visitor.accept(await(pending.removeFirst()));
            // The consumed artifact list is released before a replacement is queued.
            enqueue();
        }
    }

    private void enqueue() {
        if (!remaining.hasNext()) return;
        Input input = remaining.next();
        boolean retain = retainProjectBytes.get(input.original());
        pending.addLast(executor.submit(() -> new Prepared(input, inventory.definitions(input.original(), retain))));
    }

    private static Prepared await(Future<Prepared> future) throws IOException {
        try {
            return future.get();
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new IOException("Artifact preparation interrupted", error);
        } catch (ExecutionException error) {
            if (error.getCause() instanceof IOException cause) throw cause;
            throw new IOException("Artifact preparation failed", error.getCause());
        }
    }

    @Override public void close() {
        for (Future<Prepared> future : pending) future.cancel(true);
        executor.shutdownNow();
        executor.close();
    }
}
