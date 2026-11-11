//! Compiler-owned file facts; derived layer assignments never enter analyzer graphs.
use serde::{Deserialize, Serialize};

pub const FACTS_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureFacts {
    pub version: u32,
    pub files: Vec<FileFacts>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileFacts {
    pub build: String,
    pub project: String,
    pub file: String,
    pub language: String,
    pub namespaces: Vec<String>,
    pub names: Vec<String>,
    pub kinds: Vec<String>,
    pub annotations: Vec<String>,
    pub method_annotations: Vec<String>,
    pub base_types: Vec<String>,
    pub annotated_interfaces: Vec<String>,
    pub is_interface: bool,
    pub content: String,
    pub is_test: bool,
}

pub fn file_key(build: &str, project: &str, file: &str) -> String {
    serde_json::to_string(&[build, project, file]).expect("string arrays serialize")
}
