package fixture.spring;

import fixture.spring.events.Created;

/** A matching method name does not make this a Spring publisher. */
public final class PretendPublisher {
    public void publishEvent(Created event) { Sink.record("pretend publisher"); }
}
