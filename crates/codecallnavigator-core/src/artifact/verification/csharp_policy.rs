//! Legacy C# graphs have a known-incomplete timestamp-sensitive import manifest.
use crate::artifact::provenance::{Analysis, CSHARP_INPUT_POLICY};
use serde_json::Value;
use std::path::Path;

pub(super) fn legacy(graph: &Value, analysis: &Analysis) -> bool {
    let csharp_nodes = graph["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().any(|node| node["language"] == "csharp"));
    let csharp_targets = analysis.targets.iter().any(|target| csharp_target(target));
    if !csharp_nodes && !csharp_targets {
        return false;
    }
    let versions = serde_json::from_str::<Value>(&analysis.engine_versions).unwrap_or(Value::Null);
    versions["csharp_input_policy"].as_str() != Some(CSHARP_INPUT_POLICY)
}

fn csharp_target(target: &str) -> bool {
    Path::new(target)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "csproj" | "sln" | "slnx"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn analysis(targets: &[&str], versions: Value) -> Analysis {
        Analysis {
            version: 1,
            targets: targets.iter().map(|value| (*value).into()).collect(),
            selection: None,
            refresh_inputs: None,
            roots: vec![],
            engine_versions: versions.to_string(),
            inputs: vec![],
            limitations: vec![],
        }
    }

    #[test]
    fn csharp_nodes_or_empty_csharp_targets_require_the_supported_policy() {
        let nodes = json!({"nodes":[{"language":"csharp"}]});
        assert!(legacy(&nodes, &analysis(&[], json!({}))));
        assert!(!legacy(
            &nodes,
            &analysis(&[], json!({"csharp_input_policy":CSHARP_INPUT_POLICY}))
        ));
        assert!(legacy(
            &nodes,
            &analysis(&[], json!({"csharp_input_policy":"future-policy"}))
        ));
        assert!(legacy(
            &json!({"nodes":[]}),
            &analysis(&["@repo/Empty.CSPROJ"], json!({}))
        ));
        assert!(legacy(
            &json!({"nodes":[]}),
            &analysis(&["@repo/Empty.slnx"], json!({}))
        ));
    }

    #[test]
    fn global_callatlas_versions_do_not_invalidate_other_languages() {
        for language in ["java", "typescript"] {
            let nodes = json!({"nodes":[{"language":language}]});
            assert!(!legacy(
                &nodes,
                &analysis(&["@repo/pom.xml"], json!({"callatlas":"present"}))
            ));
        }
    }
}
