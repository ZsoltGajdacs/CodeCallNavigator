//! Graph-only display text, independent of stable identities and analyzer formats.
use codecallnavigator::{
    model::{Node, Visibility},
    search::class_name,
};

pub(super) struct Label {
    pub text: String,
    pub description: String,
}

fn access(visibility: Visibility) -> (&'static str, &'static str) {
    match visibility {
        Visibility::Public => ("+", "public"),
        Visibility::Private => ("−", "private"),
        Visibility::Protected => ("#", "protected"),
        Visibility::Package => ("~", "package-private"),
        Visibility::Internal => ("~", "internal"),
        Visibility::ProtectedInternal => ("#~", "protected internal"),
        Visibility::PrivateProtected => ("−#", "private protected"),
    }
}

fn format(name: &str, full: &str, visibility: Option<Visibility>) -> Label {
    match visibility {
        Some(visibility) => {
            let (sign, description) = access(visibility);
            Label {
                text: format!("{sign} {name}"),
                description: format!("{description} · {full}"),
            }
        }
        None => Label {
            text: name.into(),
            description: full.into(),
        },
    }
}

pub(super) fn method(node: &Node) -> Label {
    let full = if node.signature.is_empty() {
        &node.label
    } else {
        &node.signature
    };
    format(&node.label, full, node.visibility)
}

pub(super) fn class(node: &Node) -> Label {
    format(
        class_name(node),
        &node.container,
        if node.language.eq_ignore_ascii_case("typescript") {
            None
        } else {
            node.container_visibility
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecallnavigator::search::simple_class_name;

    #[test]
    fn visibility_signs_have_precise_descriptions() {
        for (visibility, sign, description) in [
            (Visibility::Public, "+", "public"),
            (Visibility::Private, "−", "private"),
            (Visibility::Protected, "#", "protected"),
            (Visibility::Package, "~", "package-private"),
            (Visibility::Internal, "~", "internal"),
            (Visibility::ProtectedInternal, "#~", "protected internal"),
            (Visibility::PrivateProtected, "−#", "private protected"),
        ] {
            let label = format("run", "app.Worker::run()", Some(visibility));
            assert_eq!(label.text, format!("{sign} run"));
            assert_eq!(
                label.description,
                format!("{description} · app.Worker::run()")
            );
        }
    }

    #[test]
    fn names_preserve_generic_arguments_friendly_lambdas_and_file_groups() {
        assert_eq!(simple_class_name("app.Outer$Inner"), "Inner");
        assert_eq!(simple_class_name("app.Outer+Inner"), "Inner");
        assert_eq!(
            simple_class_name("app.Worker<System.String>"),
            "Worker<System.String>"
        );
        let node = Node {
            label: "validate · lambda at L12".into(),
            signature: "app.Worker:lambda$1()".into(),
            ..Default::default()
        };
        assert_eq!(method(&node).text, node.label);
        assert_eq!(method(&node).description, node.signature);
        let file = Node {
            language: "typescript".into(),
            container: "src/main.test.ts".into(),
            ..Default::default()
        };
        assert_eq!(class(&file).text, file.container);
    }
}
