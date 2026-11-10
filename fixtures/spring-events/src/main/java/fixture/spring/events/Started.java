package fixture.spring.events;

import org.springframework.context.ApplicationEvent;

/** An ApplicationEvent is published directly, without a payload wrapper. */
public final class Started extends ApplicationEvent {
    public Started(Object source) { super(source); }
}
