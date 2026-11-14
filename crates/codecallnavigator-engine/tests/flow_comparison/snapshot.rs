//! One explicit analyzed-source fixture; no compiler or working source file is needed.
use codecallnavigator_core::{
    architecture::{ArchitectureFacts, FileFacts},
    artifact::{self, Artifact},
    flow::*,
    model::{GraphData, Node},
};
use serde_json::json;
use std::path::Path;
const METHOD: &str = "int same(){ return target(); }";
pub(super) struct Snapshot<'a> {
    pub root: &'a Path,
    pub marker: &'a str,
    pub extras: &'a [&'a str],
}
struct Location {
    file: String,
    build: String,
}
impl Snapshot<'_> {
    pub fn artifact(&self) -> Artifact {
        let location = Location {
            file: self.root.join("F.java").to_string_lossy().into_owned(),
            build: self.root.join("pom.xml").to_string_lossy().into_owned(),
        };
        let source = self.source();
        let graph = GraphData {
            nodes: self.nodes(&location),
            flow_inventory: Some(FlowInventory {
                version: 1,
                functions: vec![self.function(&location, &source)],
            }),
            architecture: Some(ArchitectureFacts {
                version: 1,
                files: vec![FileFacts {
                    build: location.build.clone(),
                    file: location.file.clone(),
                    project: "Fixture".into(),
                    language: "java".into(),
                    content: source,
                    ..Default::default()
                }],
            }),
            ..Default::default()
        };
        let mut saved =
            artifact::canonical(&graph, &[("repo".into(), self.root.to_owned())]).unwrap();
        saved.graph["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["label"] == "target")
            .unwrap()["label"] = json!(format!("target {}", self.marker));
        saved
    }
    fn source(&self) -> String {
        let extra = self
            .extras
            .iter()
            .map(|name| format!("int {name}(){{return 3;}}"))
            .collect::<String>();
        format!(
            "\u{feff}// {} é😀\r\nclass App {{ {METHOD} int target(){{return 2;}} {extra} }}\r\n",
            self.marker
        )
    }
    fn nodes(&self, location: &Location) -> Vec<Node> {
        ["same", "target"]
            .into_iter()
            .chain(self.extras.iter().copied())
            .map(|name| Node {
                id: name.into(),
                label: name.into(),
                signature: format!("App::{name}()"),
                container: "App".into(),
                project: "Fixture".into(),
                build: location.build.clone(),
                file: location.file.clone(),
                line: 2,
                language: "java".into(),
                kind: "method".into(),
                ..Default::default()
            })
            .collect()
    }
    fn function(&self, location: &Location, source: &str) -> FunctionFlowIR {
        let call_span = span(source, "target()");
        let call = CallSite {
            span: call_span,
            label: "target()".into(),
            targets: vec![CallTarget {
                symbol: "target".into(),
                kind: "call".into(),
                certainty: "possible".into(),
                provenance: "saved compiler candidate".into(),
            }],
            diagnostic: None,
        };
        let value = Expression::Value {
            text: "target()".into(),
            span: call_span,
            regions: vec![],
            calls: vec![call],
        };
        FunctionFlowIR {
            symbol: "same".into(),
            file: location.file.clone(),
            build: location.build.clone(),
            project: "Fixture".into(),
            language: "java".into(),
            span: span(source, METHOD),
            body: vec![Statement {
                span: span(source, "return target();"),
                kind: StatementKind::Return { value: Some(value) },
            }],
            diagnostics: vec![],
        }
    }
}
fn span(source: &str, segment: &str) -> SourceSpan {
    let start = source.find(segment).unwrap();
    SourceSpan {
        start,
        end: start + segment.len(),
    }
}
