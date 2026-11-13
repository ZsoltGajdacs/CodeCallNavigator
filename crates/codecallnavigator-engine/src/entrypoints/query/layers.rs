//! Configuration observation and cached classification stay with the CLI backend.
use super::Loaded;
use codecallnavigator_core::{
    architecture::{self, LayerAssignments, Selection, file_key},
    graph::{Filter, GraphIndex},
    query::{self, Error, RenderLayer, SelectionKind},
};
use serde_json::{Value, json};
use std::{collections::HashSet, path::Path, sync::Arc};

#[derive(Clone)]
pub(super) struct Resolved {
    selection: Selection,
    signature: String,
    assignments: Option<Arc<LayerAssignments>>,
}

pub(super) fn architecture(
    loaded: &Loaded,
    selection: Option<Selection>,
    previous: Option<&str>,
    configuration_root: Option<&Path>,
) -> query::Result<Value> {
    let resolved = resolve(loaded, selection, configuration_root)?;
    let changed = Some(resolved.signature.as_str()) != previous;
    let assignments = if changed { resolved.assignments } else { None };
    Ok(
        json!({"selection":resolved.selection,"signature":resolved.signature,"changed":changed,"assignments":assignments}),
    )
}

fn automatic(root: &Path) -> Selection {
    if root.join(".codecallnavigator/layers.yml").is_file()
        || root.join(".interactive-review/layers.yml").is_file()
    {
        Selection::Repository
    } else {
        Selection::None
    }
}

fn resolve(
    loaded: &Loaded,
    selection: Option<Selection>,
    configuration_root: Option<&Path>,
) -> query::Result<Resolved> {
    let root =
        configuration_root.unwrap_or(&loaded.roots.last().expect("project root is appended").1);
    let selection = selection.unwrap_or_else(|| automatic(root));
    if selection == Selection::None {
        return Ok(Resolved {
            selection,
            signature: "None".into(),
            assignments: None,
        });
    }
    let config = architecture::load_readonly(root, selection).map_err(layer_error)?;
    let signature = serde_json::to_string(&(
        selection,
        &config.config,
        &config.origin,
        &config.config.root,
    ))
    .map_err(layer_error)?;
    if let Some(cached) = loaded
        .layers
        .lock()
        .map_err(layer_error)?
        .as_ref()
        .filter(|cached| cached.signature == signature)
    {
        return Ok(cached.clone());
    }
    let assignments = architecture::classify(&loaded.graph()?.data, &config.config, config.origin)
        .map_err(layer_error)?;
    let resolved = Resolved {
        selection,
        signature,
        assignments: Some(Arc::new(assignments)),
    };
    *loaded.layers.lock().map_err(layer_error)? = Some(resolved.clone());
    Ok(resolved)
}

fn layer_error(error: impl std::fmt::Display) -> Error {
    Error::new("architecture_unavailable", error.to_string())
}

pub(super) struct RenderRequest<'a> {
    pub selection: &'a SelectionKind,
    pub filter: &'a Filter,
    pub layer: &'a RenderLayer,
}

pub(super) fn render(loaded: &Loaded, request: RenderRequest<'_>) -> query::Result<Value> {
    let resolved = resolve(
        loaded,
        request.layer.selection,
        request.layer.configuration_root.as_deref(),
    )?;
    if request
        .layer
        .expected_signature
        .as_deref()
        .is_some_and(|expected| expected != resolved.signature)
    {
        return Err(Error::new(
            "architecture_changed",
            "Layer configuration changed; reload assignments before combining render results",
        ));
    }
    let mut selected = query::select_render(loaded.graph()?, request.selection, request.filter)?;
    let available = resolved.assignments.is_some();
    match resolved.assignments.as_deref() {
        Some(assignments) => retain_layer(
            loaded.graph()?,
            &mut selected,
            assignments,
            request.layer.tab.as_deref(),
        ),
        None => {
            selected.nodes.clear();
            selected.edges.clear();
            selected.roots.clear();
        }
    }
    let mut result = query::render_section(loaded.graph()?, selected, request.filter)?;
    result["architecture_signature"] = json!(resolved.signature);
    if !available {
        result["message"] = json!(
            "Choose a layer preset or repository configuration. Refresh an older graph to collect architectural facts."
        );
    }
    Ok(result)
}

fn retain_layer(
    graph: &GraphIndex,
    selection: &mut query::Selection,
    assignments: &LayerAssignments,
    tab: Option<&str>,
) {
    let before_nodes = selection.nodes.len();
    let before_edges = selection.edges.len();
    selection.nodes.retain(|id| {
        let node = &graph.data.nodes[*id as usize];
        let name = assignments
            .files
            .get(&file_key(&node.build, &node.project, &node.file))
            .map(|assignment| assignment.layer.as_str())
            .unwrap_or("Other");
        assignments
            .layers
            .iter()
            .find(|layer| layer.name == name)
            .and_then(|layer| layer.tab.as_deref())
            == tab
    });
    let included: HashSet<_> = selection.nodes.iter().copied().collect();
    selection.edges.retain(|id| {
        let edge = &graph.data.edges[*id as usize];
        included.contains(&edge.from) && included.contains(&edge.to)
    });
    selection.roots.retain(|id| included.contains(id));
    selection.omitted_nodes += before_nodes - selection.nodes.len();
    selection.omitted_edges += before_edges - selection.edges.len();
}
