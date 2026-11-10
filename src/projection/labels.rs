//! Graph-only display text, independent of stable identities and analyzer formats.
use codegraph::model::{Node, Visibility};

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
    // TypeScript containers represent source files, not class declarations.
    if node.language.eq_ignore_ascii_case("typescript") {
        return format(&node.container, &node.container, None);
    }
    format(
        simple_class_name(&node.container),
        &node.container,
        node.container_visibility,
    )
}

fn simple_class_name(container: &str) -> &str {
    let mut depth = 0_u32;
    let mut start = 0;
    for (index, character) in container.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            '.' | '+' | '$' if depth == 0 => start = index + 1,
            _ => {}
        }
    }
    &container[start..]
}

#[cfg(test)]
mod tests {
    use super::*;

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
