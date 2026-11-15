//! Expansion is optional; only its exact generation-bound version enables the controls.
use serde_json::Value;
pub(super) fn supported(capabilities: &Value, operations: &[Value]) -> bool {
    let expansion = &capabilities["flow_expansion"];
    let version = expansion["version"] == 1 && expansion["expected_generation"] == true;
    let operation = operations.iter().any(|name| name == "flow_expansion");
    let bound = capabilities["generation_required"]
        .as_array()
        .is_some_and(|required| required.iter().any(|name| name == "flow_expansion"));
    version && operation && bound
}
