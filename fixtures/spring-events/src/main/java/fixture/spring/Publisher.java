package fixture.spring;

import fixture.spring.events.Created;
import fixture.spring.events.Started;
import org.springframework.context.ApplicationEventPublisher;

public final class Publisher {
    public void publishPayload(ApplicationEventPublisher publisher) {
        publisher.publishEvent(new Created("enabled"));
    }

    public void publishPayloadTwice(ApplicationEventPublisher publisher) {
        publisher.publishEvent(new Created("first"));
        publisher.publishEvent(new Created("second"));
    }

    public void publishApplicationEvent(ApplicationEventPublisher publisher) {
        publisher.publishEvent(new Started(this));
    }

    public void publishErased(ApplicationEventPublisher publisher, Object event) {
        publisher.publishEvent(event);
    }

    public <T> void publishGeneric(ApplicationEventPublisher publisher, T event) {
        publisher.publishEvent(event);
    }

    public void sameNamedApi(PretendPublisher publisher) {
        publisher.publishEvent(new Created("not a Spring publication"));
    }
}
