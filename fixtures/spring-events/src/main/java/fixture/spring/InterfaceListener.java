package fixture.spring;

import fixture.spring.events.Started;
import org.springframework.context.ApplicationListener;
import org.springframework.stereotype.Component;

@Component
public final class InterfaceListener implements ApplicationListener<Started> {
    @Override
    public void onApplicationEvent(Started event) { Sink.record("typed interface listener"); }
}
