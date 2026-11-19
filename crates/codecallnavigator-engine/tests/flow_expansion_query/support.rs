use codecallnavigator_core::{
    architecture::{ArchitectureFacts, FileFacts},
    artifact,
    flow::*,
    model::{GraphData, Node},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};
pub struct Saved {
    directory: tempfile::TempDir,
    pub path: PathBuf,
    pub root: String,
    pub digest: String,
    pub saved: artifact::Artifact,
}
impl Saved {
    pub fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path();
        let saved =
            artifact::canonical(&graph(project), &[("repo".into(), project.into())]).unwrap();
        let root = saved.graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["label"] == "root")
            .unwrap()["id"]
            .as_str()
            .unwrap()
            .into();
        let mut fixture = Self {
            directory,
            path: PathBuf::new(),
            root,
            digest: String::new(),
            saved,
        };
        fixture.write();
        fixture
    }
    fn write(&mut self) {
        artifact::publish(
            self.directory.path(),
            &self.saved,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
        self.path = artifact::working_manifest_path(self.directory.path()).unwrap();
        let bytes = fs::read(&self.path).unwrap();
        self.digest = blake3::hash(&bytes).to_hex().to_string();
    }
    pub fn with_content(mut self, content: String) -> Self {
        for file in self.saved.graph["architecture"]["files"]
            .as_array_mut()
            .unwrap()
        {
            file["content"] = json!(content);
        }
        self.write();
        self
    }
    pub fn operation(&self, selections: Vec<Value>) -> Value {
        json!({"kind":"flow_expansion","expansion_version":1,"symbol":self.root,"selections":selections})
    }
    pub fn selection(&self) -> Value {
        let raw = self.saved.graph["flow_inventory"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["symbol"] == self.root)
            .unwrap();
        let function: FunctionFlowIR = serde_json::from_value(raw.clone()).unwrap();
        let sites = call_sites(&function);
        json!({"caller":[],"choice":{"site":sites[0].address,"target":sites[0].site.targets[0]}})
    }
    pub fn ask(&self, query: Value, generation: Option<&str>) -> Value {
        self.exchange(query, generation, "query")
    }
    pub fn ask_session(&self, query: Value) -> Value {
        self.exchange(query, Some(&self.digest), "query-session")
    }
    fn exchange(&self, query: Value, generation: Option<&str>, transport: &str) -> Value {
        let mut child = Command::new(env!("CARGO_BIN_EXE_codecallnavigator-engine"))
            .arg(transport)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(child.stdin.take().unwrap(), "{}", json!({"version":2,"project":self.directory.path(),"expected_generation":generation,"query":query})).unwrap();
        let output = child.wait_with_output().unwrap();
        serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&output.stderr)))
    }
}

fn graph(project: &std::path::Path) -> GraphData {
    let build = project.join("App.csproj").to_string_lossy().into_owned();
    let mut graph = GraphData::default();
    for id in ["root", "leaf"] {
        let file = project
            .join(format!("{id}.cs"))
            .to_string_lossy()
            .into_owned();
        graph.nodes.push(Node {
            id: id.into(),
            label: id.into(),
            signature: id.into(),
            file: file.clone(),
            build: build.clone(),
            project: "App".into(),
            language: "csharp".into(),
            kind: "method".into(),
            ..Default::default()
        });
        graph
            .architecture
            .get_or_insert_with(|| ArchitectureFacts {
                version: 1,
                files: vec![],
            })
            .files
            .push(FileFacts {
                file: file.clone(),
                build: build.clone(),
                project: "App".into(),
                language: "csharp".into(),
                content: "leaf()".into(),
                ..Default::default()
            });
        let body = if id == "root" {
            json!([{"kind":"evaluate","value":{"kind":"value","text":"leaf()","span":{"start":0,"end":6},"calls":[{"label":"leaf()","span":{"start":0,"end":6},"invocation":{"timing":"synchronous","dispatch":"direct","isolated_ordered":true},"targets":[{"symbol":"leaf","certainty":"possible","provenance":"saved fixture"}]}]}}])
        } else {
            json!([{"kind":"action","text":"child body"}])
        };
        let function=serde_json::from_value(json!({"symbol":id,"file":file,"build":build,"project":"App","language":"csharp","execution":"synchronous","span":{"start":0,"end":6},"body":body})).unwrap();
        graph
            .flow_inventory
            .get_or_insert_with(|| FlowInventory {
                version: 1,
                functions: vec![],
            })
            .functions
            .push(function);
    }
    graph
}
