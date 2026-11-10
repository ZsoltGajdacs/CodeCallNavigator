package fixture.spring.events;

/** A plain payload that Spring wraps in PayloadApplicationEvent. */
public record Created(String id) {}
