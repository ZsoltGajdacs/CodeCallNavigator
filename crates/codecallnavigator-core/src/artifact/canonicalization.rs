//! Canonical ordering is scoped to explicit contract locations; extensions stay opaque.
use anyhow::Result;
use serde_json::Value;

pub(super) fn sort_graph(graph: &mut Value) -> Result<()> {
    sort_fields(
        graph,
        &[
            "nodes",
            "identity_roots",
            "folder_project_builds",
            "roots",
            "diagnostics",
        ],
    );
    each(graph, "edges", edge);
    sort_fields(graph, &["edges"]);
    if let Some(entries) = graph.get_mut("entry_points") {
        sort_fields(
            entries,
            &["candidates", "application_starts", "framework_entries"],
        );
    }
    if let Some(boundaries) = graph.get_mut("boundaries") {
        each(boundaries, "routes", |route| {
            sort_fields(route, &["methods", "restrictions"])
        });
        sort_fields(boundaries, &["routes", "uses"]);
    }
    if let Some(architecture) = graph.get_mut("architecture") {
        each(architecture, "files", architecture_file);
        sort_fields(architecture, &["files"]);
    }
    if let Some(inventory) = graph.get_mut("flow_inventory") {
        each(inventory, "functions", function);
        sort_fields(inventory, &["functions"]);
    }
    if let Some(analysis) = graph.get_mut("analysis") {
        super::provenance::sort_analysis(analysis)?;
    }
    Ok(())
}

fn edge(edge: &mut Value) {
    each(edge, "evidence", |evidence| {
        sort_fields(evidence, &["supporting_symbols"])
    });
    sort_fields(edge, &["evidence"]);
}

fn architecture_file(file: &mut Value) {
    sort_fields(
        file,
        &[
            "namespaces",
            "names",
            "kinds",
            "annotations",
            "method_annotations",
            "base_types",
            "annotated_interfaces",
        ],
    );
}

fn function(function: &mut Value) {
    sort_fields(function, &["diagnostics"]);
    each(function, "body", statement);
}

fn statement(statement: &mut Value) {
    match statement.get("kind").and_then(Value::as_str) {
        Some("evaluate" | "throw" | "return" | "yield") => expression_field(statement, "value"),
        Some("action") => calls(statement),
        Some("if") => {
            expression_field(statement, "condition");
            each(statement, "then_branch", self::statement);
            each(statement, "else_branch", self::statement);
        }
        Some("loop") => {
            expression_field(statement, "condition");
            each(statement, "body", self::statement);
            each(statement, "update", self::statement);
        }
        Some("switch") => switch(statement),
        Some("try") => try_statement(statement),
        Some("block" | "region") => each(statement, "body", self::statement),
        _ => {}
    }
}

fn try_statement(statement: &mut Value) {
    for field in ["resources", "body", "finally", "cleanup"] {
        each(statement, field, self::statement);
    }
    each(statement, "catches", |catch| {
        each(catch, "body", self::statement)
    });
}

fn expression_field(value: &mut Value, field: &str) {
    if let Some(expression) = value.get_mut(field) {
        self::expression(expression);
    }
}

fn expression(expression: &mut Value) {
    match expression.get("kind").and_then(Value::as_str) {
        Some("sequence") => each(expression, "items", self::expression),
        Some("switch") => switch(expression),
        Some("value") => {
            calls(expression);
            each(expression, "regions", statement);
        }
        Some("and" | "or") => {
            expression_field(expression, "left");
            expression_field(expression, "right");
        }
        Some("conditional") => {
            for field in ["condition", "when_true", "when_false"] {
                expression_field(expression, field);
            }
        }
        _ => {}
    }
}

fn switch(value: &mut Value) {
    expression_field(value, "subject");
    each(value, "cases", |case| {
        expression_field(case, "test");
        each(case, "body", statement);
    });
}

fn calls(value: &mut Value) {
    each(value, "calls", |call| sort_fields(call, &["targets"]));
}

fn each(value: &mut Value, field: &str, mut visit: impl FnMut(&mut Value)) {
    if let Some(items) = value.get_mut(field).and_then(Value::as_array_mut) {
        for item in items {
            visit(item);
        }
    }
}

fn sort_fields(value: &mut Value, fields: &[&str]) {
    for field in fields {
        if let Some(items) = value.get_mut(field).and_then(Value::as_array_mut) {
            items.sort_by_cached_key(Value::to_string);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn known_sets_sort_without_reordering_control_flow_or_extension_arrays() {
        let opaque = json!({"targets":["z","a"], "body":[{"kind":"action","calls":[{"targets":["z","a"]}]}]});
        let mut graph = json!({
            "nodes":[],"edges":[{"evidence":[{"supporting_symbols":["z","a"],"extension":opaque}]}],
            "flow_inventory":{"functions":[{"symbol":"f","body":[
                {"kind":"action","text":"first","calls":[{"targets":[{"symbol":"z"},{"symbol":"a"}],"extension":opaque}],"extension":opaque},
                {"kind":"if","condition":{"kind":"value","calls":[{"targets":[{"symbol":"z"},{"symbol":"a"}]}]},"then_branch":[{"kind":"return","value":null}]},
                {"kind":"action","text":"last","calls":[]}
            ]}]},
            "extension":opaque
        });
        sort_graph(&mut graph).unwrap();
        let body = &graph["flow_inventory"]["functions"][0]["body"];
        assert_eq!(body[0]["text"], "first");
        assert_eq!(body[2]["text"], "last");
        assert_eq!(body[0]["calls"][0]["targets"][0]["symbol"], "a");
        assert_eq!(
            body[1]["condition"]["calls"][0]["targets"][0]["symbol"],
            "a"
        );
        assert_eq!(body[0]["extension"], opaque);
        assert_eq!(body[0]["calls"][0]["extension"], opaque);
        assert_eq!(graph["extension"], opaque);
        assert_eq!(
            graph["edges"][0]["evidence"][0]["supporting_symbols"],
            json!(["a", "z"])
        );
    }
    #[test]
    fn provenance_order_is_idempotent_when_exclusions_start_unsorted() {
        let mut graph = json!({"analysis":{
            "version":1,"targets":[],"roots":["repo"],"engine_versions":"",
            "limitations":[],"inputs":[
                {"path":"@repo/a","kind":"directory","hash":"a".repeat(64),"excluded_children":["z","a"]},
                {"path":"@repo/b","kind":"directory","hash":"b".repeat(64),"excluded_children":["b","c"]}
            ]
        }});
        sort_graph(&mut graph).unwrap();
        let first = graph.clone();
        sort_graph(&mut graph).unwrap();
        assert_eq!(graph, first);
        assert_eq!(graph["analysis"]["inputs"][0]["path"], "@repo/a");
    }
}
