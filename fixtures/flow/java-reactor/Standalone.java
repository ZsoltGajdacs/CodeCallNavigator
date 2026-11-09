package fixture;
import reactor.core.publisher.Mono;
import reactor.core.publisher.Flux;
import java.util.List;
public final class Standalone {
 static final StringBuilder events = new StringBuilder();
 static int leaf(int value) { events.append(value).append(","); return value; }
 static Mono<Integer> pipeline() {
   return Mono.defer(() -> Mono.just(leaf(1)))
     .map(value -> leaf(value)).filter(value -> value > 0)
     .switchIfEmpty(Mono.just(leaf(2)))
     .onErrorResume(error -> Mono.just(leaf(3))).then(Mono.just(leaf(4)));
 }
 static Mono<List<Integer>> stream() { return Flux.just(1).flatMap(value -> Mono.just(leaf(value))).collectList(); }
 static Mono<Void> concurrent() { return Mono.when(Mono.just(1), Mono.zip(Mono.just(2), Mono.just(3))); }
 static class Lookalike { Lookalike map(java.util.function.Function<Integer,Integer> fn) { return this; } }
 static Lookalike ordinary() { return new Lookalike().map(value -> leaf(value)); }
 public static void main(String[] arguments) {
   Mono<Integer> work = pipeline();
   String assembly = events.toString();
   int result = work.block();
   System.out.println(assembly + "|" + events + "|" + result);
 }
}
