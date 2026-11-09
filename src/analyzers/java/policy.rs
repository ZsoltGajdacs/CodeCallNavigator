//! Java declaration eligibility; independent of execution, parsing and graph mutation.
use super::{metadata::MethodMetadata, source_inventory::SourceInventory};

pub(super) fn excluded(method: &MethodMetadata, inventory: &SourceInventory) -> bool {
    let name = method
        .signature
        .split_once(':')
        .map(|(_, method)| method)
        .unwrap_or("");
    name.starts_with("<")
        || is_object_method(name)
        || method.flags & 0x0040 != 0
        || !(inventory.declares(&method.signature, &method.return_type)
            || authored_lambda(method, inventory))
}

fn authored_lambda(method: &MethodMetadata, inventory: &SourceInventory) -> bool {
    // javac lambda implementations are synthetic but represent authored source behavior.
    // Without debug lines, source presence and javac's reserved implementation name identify them.
    method.flags & 0x1000 != 0
        && method
            .signature
            .split_once(':')
            .is_some_and(|(_, name)| name.starts_with("lambda$"))
        && inventory.has_lambda(&method.class.name, method.start_line)
}

fn is_object_method(signature: &str) -> bool {
    matches!(
        signature,
        "toString()"
            | "hashCode()"
            | "equals(java.lang.Object)"
            | "getClass()"
            | "clone()"
            | "finalize()"
            | "notify()"
            | "notifyAll()"
            | "wait()"
            | "wait(long)"
            | "wait(long,int)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn object_signatures_do_not_remove_application_overloads() {
        for signature in [
            "toString()",
            "hashCode()",
            "equals(java.lang.Object)",
            "wait(long,int)",
        ] {
            assert!(is_object_method(signature));
        }
        for signature in [
            "toString(int)",
            "hashCode(java.lang.String)",
            "equals(fixture.Value)",
            "await()",
        ] {
            assert!(!is_object_method(signature));
        }
    }
}
