package fixture.spring.http;

import fixture.spring.Sink;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api")
public final class ConstrainedController {
    @GetMapping(path = "/ambiguous", produces = "application/json")
    public String json() { Sink.record("json variant"); return "{}"; }

    @GetMapping(path = "/ambiguous", produces = "text/plain")
    public String text() { Sink.record("text variant"); return "text"; }

    @GetMapping("/constrained")
    public String unconstrained() { Sink.record("unconstrained variant"); return "default"; }

    @GetMapping(path = "/constrained", params = "mode=special")
    public String constrained() { Sink.record("query constrained variant"); return "special"; }
}
