//! Path and identity conversion visits only fields owned by the portable contract.
use super::{
    identity,
    paths::{Paths, ProjectOwner},
    portable::PortableGraph,
};
use crate::{
    flow,
    model::{RelationshipEvidence, SourceSite},
};
use anyhow::Result;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(super) fn transform(
    graph: &mut PortableGraph,
    paths: &Paths,
    identities: Option<&HashMap<String, String>>,
    hydrate: bool,
) -> Result<()> {
    let transform = Transform {
        paths,
        identities,
        hydrate,
        folder_builds: graph.folder_project_builds.iter().cloned().collect(),
    };
    for node in &mut graph.nodes {
        transform.node(node)?;
    }
    transform.edges(&mut graph.edges)?;
    if let Some(entries) = &mut graph.entry_points {
        transform.entries(entries)?;
    }
    if let Some(boundaries) = &mut graph.boundaries {
        transform.boundaries(boundaries)?;
    }
    if let Some(facts) = &mut graph.architecture {
        transform.architecture(facts)?;
    }
    if let Some(inventory) = &mut graph.flow_inventory {
        transform.inventory(inventory)?;
    }
    for root in graph
        .roots
        .iter_mut()
        .chain(&mut graph.identity_roots)
        .chain(&mut graph.folder_project_builds)
    {
        transform.path(root)?;
    }
    transform.diagnostics(&mut graph.diagnostics);
    Ok(())
}
struct Transform<'a> {
    paths: &'a Paths,
    identities: Option<&'a HashMap<String, String>>,
    hydrate: bool,
    folder_builds: HashSet<String>,
}
struct Owner<'a> {
    build: &'a mut String,
    project: &'a mut String,
    file: &'a mut String,
    language: &'a str,
}
impl Transform<'_> {
    fn edges(&self, edges: &mut [super::portable::PortableEdge]) -> Result<()> {
        for edge in edges {
            for evidence in &mut edge.evidence {
                self.evidence(evidence)?;
            }
        }
        Ok(())
    }
    fn architecture(&self, facts: &mut crate::architecture::ArchitectureFacts) -> Result<()> {
        for file in &mut facts.files {
            self.owner(Owner {
                build: &mut file.build,
                project: &mut file.project,
                file: &mut file.file,
                language: &file.language,
            })?;
        }
        Ok(())
    }
    fn inventory(&self, inventory: &mut flow::FlowInventory) -> Result<()> {
        for function in &mut inventory.functions {
            self.function(function)?;
        }
        Ok(())
    }
    fn node(&self, node: &mut crate::model::Node) -> Result<()> {
        if node.language == "typescript" {
            self.display(node)?;
        }
        self.owner(Owner {
            build: &mut node.build,
            project: &mut node.project,
            file: &mut node.file,
            language: &node.language,
        })?;
        self.symbol(&mut node.id)
    }
    fn display(&self, node: &mut crate::model::Node) -> Result<()> {
        if let Some(source) = self
            .paths
            .source_display(&node.file, &node.build, self.hydrate)?
        {
            update(&mut node.container, |value| {
                source.rewrite("container", value);
                Ok(())
            })?;
            update(&mut node.namespace, |value| {
                source.rewrite("namespace", value);
                Ok(())
            })?;
        }
        Ok(())
    }
    fn owner(&self, owner: Owner<'_>) -> Result<()> {
        let directory = self.folder_builds.contains(owner.build);
        if directory || owner.language == "typescript" {
            update(owner.project, |value| {
                self.paths.project(
                    value,
                    ProjectOwner {
                        build: owner.build,
                        directory,
                    },
                    self.hydrate,
                )
            })?;
        }
        self.path(owner.build)?;
        self.path(owner.file)
    }
    fn entries(&self, entries: &mut crate::model::EntryPoints) -> Result<()> {
        for entry in entries
            .candidates
            .iter_mut()
            .chain(&mut entries.framework_entries)
            .chain(&mut entries.application_starts)
        {
            self.symbol(entry)?;
        }
        Ok(())
    }
    fn boundaries(&self, boundaries: &mut crate::boundary::BoundaryInventory) -> Result<()> {
        for route in &mut boundaries.routes {
            if let Some(symbol) = &mut route.symbol {
                self.symbol(symbol)?;
            }
            self.path(&mut route.build)?;
            self.site(&mut route.site)?;
        }
        for usage in &mut boundaries.uses {
            self.symbol(&mut usage.symbol)?;
            self.path(&mut usage.build)?;
            self.site(&mut usage.site)?;
        }
        Ok(())
    }
    fn function(&self, function: &mut flow::FunctionFlowIR) -> Result<()> {
        self.owner(Owner {
            build: &mut function.build,
            project: &mut function.project,
            file: &mut function.file,
            language: &function.language,
        })?;
        self.symbol(&mut function.symbol)?;
        let mut result = Ok(());
        flow::visit_calls_mut(function, &mut |call| {
            if result.is_ok() {
                result = self.call(call);
            }
        });
        self.diagnostics(&mut function.diagnostics);
        result
    }
    fn call(&self, call: &mut flow::CallSite) -> Result<()> {
        for target in &mut call.targets {
            self.symbol(&mut target.symbol)?;
        }
        Ok(())
    }
    fn path(&self, text: &mut String) -> Result<()> {
        update(text, |value| self.paths.rewrite(value, self.hydrate))
    }
    fn symbol(&self, text: &mut String) -> Result<()> {
        if let Some(ids) = self.identities {
            update(text, |value| identity::rewrite(value, ids))?;
        }
        Ok(())
    }
    fn site(&self, site: &mut SourceSite) -> Result<()> {
        self.path(&mut site.file)
    }
    fn evidence(&self, evidence: &mut RelationshipEvidence) -> Result<()> {
        self.site(&mut evidence.invocation)?;
        if let Some(site) = &mut evidence.registration {
            self.site(site)?;
        }
        for symbol in &mut evidence.supporting_symbols {
            self.symbol(symbol)?;
        }
        Ok(())
    }
    fn diagnostics(&self, diagnostics: &mut [String]) {
        if !self.hydrate {
            for diagnostic in diagnostics {
                *diagnostic = self.paths.embedded(diagnostic);
            }
        }
    }
}
fn update(text: &mut String, operation: impl FnOnce(&mut Value) -> Result<()>) -> Result<()> {
    let mut value = Value::String(text.clone());
    operation(&mut value)?;
    *text = value
        .as_str()
        .expect("string transformations preserve string values")
        .into();
    Ok(())
}
