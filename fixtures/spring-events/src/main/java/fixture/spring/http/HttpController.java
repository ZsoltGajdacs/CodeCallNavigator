package fixture.spring.http;

import fixture.spring.Sink;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestMethod;
import org.springframework.web.bind.annotation.RestController;

@RestController
@RequestMapping("/api")
public final class HttpController {
    @GetMapping("/items")
    public String items() { Sink.record("http items"); return "items"; }

    @GetMapping("/items/{id}")
    public String item() { Sink.record("http item"); return "item"; }

    @PostMapping("/items")
    public void create() { Sink.record("http create"); }

    @RequestMapping(path = "/health", method = RequestMethod.GET)
    public String health() { Sink.record("http health"); return "ok"; }

    @fixture.spring.fake.GetMapping("/pretend")
    public void pretend() { Sink.record("custom annotation is not Spring"); }
}
