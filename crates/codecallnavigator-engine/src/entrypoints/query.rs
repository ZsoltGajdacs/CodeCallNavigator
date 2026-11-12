//! Query lifecycle: load exactly one artifact generation, then dispatch shared semantics.
mod filesystem;
use codecallnavigator_core::{
    artifact::{self, Artifact, verification},
    graph::GraphIndex,
    query::{self, Error, Request, Response},
};
use std::{
    fs,
    io::{self, Read},
    sync::Arc,
};

pub(super) struct Loaded {
    pub artifact: Artifact,
    pub graph: GraphIndex,
    pub generation: String,
    pub freshness: serde_json::Value,
    pub roots: Vec<(String, std::path::PathBuf)>,
}

pub(super) fn load(request: &Request) -> query::Result<Loaded> {
    let roots = roots(request)?;
    let project = &roots.last().expect("project root is always appended").1;
    let bytes = read_generation(project)?;
    let generation = blake3::hash(&bytes).to_hex().to_string();
    load_bytes(request, roots, &bytes, generation)
}

pub(super) fn roots(request: &Request) -> query::Result<Vec<(String, std::path::PathBuf)>> {
    let project = request
        .project
        .canonicalize()
        .map_err(|e| Error::new("project_unavailable", e.to_string()))?;
    let mut roots = request
        .roots
        .iter()
        .map(|(name, path)| {
            if name == "repo" {
                return Err(Error::new(
                    "invalid_request",
                    "Root repo is reserved for project",
                ));
            }
            Ok((
                name.clone(),
                if path.is_absolute() {
                    path.clone()
                } else {
                    project.join(path)
                },
            ))
        })
        .collect::<query::Result<Vec<_>>>()?;
    roots.push(("repo".into(), project.clone()));
    artifact::validate_roots(&roots)
        .map_err(|e| Error::new("invalid_request", format!("{e:#}")))?;
    Ok(roots)
}

pub(super) fn read_generation(project: &std::path::Path) -> query::Result<Vec<u8>> {
    fs::read(project.join(artifact::GRAPH_PATH)).map_err(|e| {
        Error::new(
            if e.kind() == io::ErrorKind::NotFound {
                "graph_missing"
            } else {
                "graph_unavailable"
            },
            e.to_string(),
        )
    })
}

pub(super) fn load_bytes(
    request: &Request,
    roots: Vec<(String, std::path::PathBuf)>,
    bytes: &[u8],
    generation: String,
) -> query::Result<Loaded> {
    check_generation(request, &generation)?;
    let artifact =
        Artifact::from_bytes(bytes).map_err(|e| Error::new("invalid_graph", format!("{e:#}")))?;
    let freshness = verification::verify_artifact(&artifact, &generation, &roots)
        .map_err(|e| Error::new("verification_failed", format!("{e:#}")))?;
    let data = artifact
        .hydrate(&roots)
        .map_err(|e| Error::new("invalid_graph", format!("{e:#}")))?;
    let graph = GraphIndex::new(Arc::new(data))
        .map_err(|e| Error::new("invalid_graph", format!("{e:#}")))?;
    Ok(Loaded {
        artifact,
        graph,
        generation,
        freshness: serde_json::to_value(freshness)
            .map_err(|e| Error::new("serialization_failed", e.to_string()))?,
        roots,
    })
}

impl Loaded {
    pub(super) fn matches_generation(
        &self,
        generation: &str,
        roots: &[(String, std::path::PathBuf)],
    ) -> bool {
        self.generation == generation && self.roots == roots
    }

    pub(super) fn refresh_freshness(&mut self) -> query::Result<()> {
        let report = verification::verify_artifact(&self.artifact, &self.generation, &self.roots)
            .map_err(|e| Error::new("verification_failed", format!("{e:#}")))?;
        self.freshness = serde_json::to_value(report)
            .map_err(|e| Error::new("serialization_failed", e.to_string()))?;
        Ok(())
    }
}

pub(super) fn validate(request: &Request) -> query::Result<()> {
    if request.version != query::VERSION {
        return Err(Error::new(
            "unsupported_version",
            "Unsupported query contract version",
        ));
    }
    if matches!(request.query, query::Query::RenderSelection { .. })
        && request.expected_generation.is_none()
    {
        return Err(Error::new(
            "invalid_request",
            "Render selection requires expected_generation from its render snapshot",
        ));
    }
    Ok(())
}

pub(super) fn check_generation(request: &Request, generation: &str) -> query::Result<()> {
    validate(request)?;
    if request
        .expected_generation
        .as_deref()
        .is_some_and(|expected| expected != generation)
    {
        return Err(Error::new(
            "generation_mismatch",
            "The saved graph changed; reload before combining results",
        ));
    }
    Ok(())
}

pub(super) fn respond(loaded: &Loaded, request: &Request) -> Response {
    let result = check_generation(request, &loaded.generation).and_then(|()| {
        if super::cancellation::CANCELLED.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Error::new("cancelled", "Query cancelled"));
        }
        match &request.query {
            query::Query::Source {
                symbol,
                origin: query::SourceOrigin::Live,
            } => filesystem::source(loaded, symbol),
            query::Query::Explorer {
                path,
                offset,
                limit,
            } => filesystem::directory(&loaded.roots, path, *offset, *limit),
            query => query::execute(&loaded.graph, query),
        }
    });
    envelope(Some(loaded), result)
}

/// Live directory navigation is available before a graph has been published.
/// An explicitly generation-bound request still follows the normal graph path.
pub(super) fn live_response(request: &Request) -> Option<Response> {
    if request.expected_generation.is_some() {
        return None;
    }
    let query::Query::Explorer {
        path,
        offset,
        limit,
    } = &request.query
    else {
        return None;
    };
    let result = check_generation(request, "").and_then(|()| {
        let roots = roots(request)?;
        filesystem::directory(&roots, path, *offset, *limit)
    });
    Some(envelope(None, result))
}

pub(super) fn envelope(
    loaded: Option<&Loaded>,
    result: query::Result<serde_json::Value>,
) -> Response {
    let (result, error) = match result {
        Ok(value) => (Some(value), None),
        Err(error) => (None, Some(error)),
    };
    Response {
        version: query::VERSION,
        generation: loaded.map(|l| l.generation.clone()),
        freshness: loaded.map(|l| l.freshness.clone()),
        result,
        error,
    }
}

pub fn run() -> anyhow::Result<i32> {
    let response = (|| {
        let mut bytes = Vec::new();
        io::stdin()
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::new("invalid_request", e.to_string()))?;
        if bytes.len() > 4 * 1024 * 1024 {
            return Err(Error::new("invalid_request", "Query request exceeds 4 MiB"));
        }
        let request: Request = serde_json::from_slice(&bytes)
            .map_err(|e| Error::new("invalid_request", e.to_string()))?;
        validate(&request)?;
        if let Some(response) = live_response(&request) {
            return Ok(response);
        }
        let loaded = load(&request)?;
        Ok(respond(&loaded, &request))
    })()
    .unwrap_or_else(|error| envelope(None, Err(error)));
    let status = if response.error.is_some() { 2 } else { 0 };
    println!("{}", serde_json::to_string(&response)?);
    Ok(status)
}
