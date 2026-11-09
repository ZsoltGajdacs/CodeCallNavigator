use codegraph::model::{relationship_is_reference, relationship_is_uncertain};

#[test]
fn references_and_certainty_have_one_shared_policy() {
    for kind in [
        "methodgroup",
        "reference",
        "callback",
        "_LM",
        "_BSM",
        "react-event",
        "react-prop",
    ] {
        assert!(relationship_is_reference(kind), "{kind}");
    }
    assert!(!relationship_is_reference("call"));
    assert!(!relationship_is_reference("react-render"));
    assert!(!relationship_is_uncertain("direct"));
    assert!(!relationship_is_uncertain("proven-direct"));
    assert!(relationship_is_uncertain("framework-possible"));
}
