use crate::graph::{Direction, Filter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub version: u32,
    pub project: PathBuf,
    #[serde(default)]
    pub roots: Vec<(String, PathBuf)>,
    #[serde(default)]
    pub expected_generation: Option<String>,
    pub query: Query,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Query {
    Status,
    /// Explicit full render snapshot; index-based edges are local to this generation only.
    Snapshot,
    Search {
        text: String,
        #[serde(default)]
        category: Category,
        #[serde(default)]
        offset: usize,
        #[serde(default = "search_limit")]
        limit: usize,
    },
    Symbol {
        symbol: String,
    },
    Source {
        symbol: String,
        #[serde(default)]
        origin: SourceOrigin,
    },
    Explorer {
        /// Absolute live directory path, restricted to configured roots by the engine.
        path: PathBuf,
        #[serde(default)]
        offset: usize,
        #[serde(default = "directory_limit")]
        limit: usize,
    },
    Selection {
        selection: SelectionKind,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
        #[serde(default)]
        limits: Limits,
    },
    /// Complete semantic membership for generation-bound desktop presentation.
    /// Traversal retains the graph engine's intrinsic safety limits.
    RenderSelection {
        selection: SelectionKind,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
    },
    Neighbors {
        symbol: String,
        #[serde(default, deserialize_with = "filter")]
        filter: Filter,
        #[serde(default)]
        limits: Limits,
    },
    Flow {
        symbol: String,
    },
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    #[default]
    All,
    Classes,
    Methods,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOrigin {
    #[default]
    Analyzed,
    Live,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SelectionKind {
    Overview,
    EntryPoints {
        direction: Direction,
        depth: u32,
    },
    Neighborhood {
        symbols: Vec<String>,
        direction: Direction,
        depth: u32,
    },
    Path {
        from: String,
        to: String,
    },
    File {
        file: String,
    },
    Class {
        build: String,
        project: String,
        container: String,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub nodes: usize,
    pub edges: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            nodes: 1500,
            edges: 6000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub version: u32,
    pub generation: Option<String>,
    pub freshness: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<Error>,
}

fn search_limit() -> usize {
    50
}
fn directory_limit() -> usize {
    1000
}

// Persisted desktop filters support a legacy upgrade policy that intentionally
// resets old relationship switches. Public requests must never silently apply
// that policy to a caller's explicit relationship selection.
fn filter<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Filter, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if value.get("version").and_then(Value::as_u64) != Some(2) {
        return Err(serde::de::Error::custom(
            "Explicit query filters require version 2 and both relationship selections",
        ));
    }
    serde_json::from_value(value).map_err(serde::de::Error::custom)
}
