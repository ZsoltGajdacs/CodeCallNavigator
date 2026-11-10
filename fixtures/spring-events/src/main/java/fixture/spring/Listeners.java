package fixture.spring;

import fixture.spring.events.Created;
import org.springframework.context.ApplicationEvent;
import org.springframework.context.PayloadApplicationEvent;
import org.springframework.context.event.EventListener;
import org.springframework.scheduling.annotation.Async;
import org.springframework.stereotype.Component;
import org.springframework.transaction.event.TransactionPhase;
import org.springframework.transaction.event.TransactionalEventListener;

@Component
public final class Listeners {
    @EventListener
    public void onPayload(Created event) { Sink.record("payload"); }

    @EventListener(classes = Created.class)
    public void onExplicitPayload() { Sink.record("explicit payload"); }

    @EventListener(condition = "#root.args[0].id == 'enabled'")
    public void onConditionalPayload(Created event) { Sink.record("conditional payload"); }

    @TransactionalEventListener(phase = TransactionPhase.AFTER_COMMIT)
    public void onCommittedPayload(Created event) { Sink.record("committed payload"); }

    @Async
    @EventListener
    public void onAsyncPayload(Created event) { Sink.record("async payload"); }

    @EventListener
    public void onPayloadEnvelope(PayloadApplicationEvent<Created> event) { Sink.record("payload envelope"); }

    @EventListener
    public void onAnyApplicationEvent(ApplicationEvent event) { Sink.record("application event"); }

    @EventListener
    public void onOtherCreated(fixture.spring.other.Created event) { Sink.record("different qualified payload"); }

    @fixture.spring.fake.EventListener
    public void onPretendAnnotation(Created event) { Sink.record("pretend annotation"); }

    public void ordinaryMethod(Created event) { Sink.record("not a listener"); }
}
