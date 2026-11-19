//! Capture a project once; current graph refreshes cannot silently retarget historical queries.
use super::{Project, Workspace};
use crate::{
    app::State,
    engine_client::{Descriptor, QueryContext, flow_comparison::ComparisonRefs},
    runtime::Runtime,
};
use std::path::Path;
impl State {
    #[cfg(test)]
    pub fn open_flow_comparison(&mut self, base: String, head: String) -> Result<(), String> {
        self.open_selected_flow_comparison(ComparisonRefs {
            base,
            head,
            ..Default::default()
        })
    }
    pub(super) fn open_selected_flow_comparison(
        &mut self,
        refs: ComparisonRefs,
    ) -> Result<(), String> {
        self.flow_residency.activity();
        let (runtime, context) = self.comparison_context()?;
        self.flow_comparison.open(Project {
            context,
            refs,
            runtime,
        });
        Ok(())
    }

    pub(super) fn library_comparison_context(&self) -> Result<QueryContext, String> {
        match &self.flow_comparison.project {
            Some(project) => Ok(project.context_for(self.flow_comparison.identity.captured())),
            None => self.comparison_context().map(|(_, context)| context),
        }
    }

    pub(super) fn comparison_context(&self) -> Result<(Runtime, QueryContext), String> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or("The standalone CLI is not ready")?;
        let context = if let Some(context) = &self.query_context {
            QueryContext::new(
                runtime,
                Descriptor {
                    project: context.descriptor.project.clone(),
                    roots: context.descriptor.roots.clone(),
                    generation: None,
                },
            )
        } else {
            let folder = self
                .project_open
                .folder
                .as_deref()
                .ok_or("Choose a project folder first")?;
            QueryContext::for_project(runtime, folder.into())
        };
        Ok((runtime.as_ref().clone(), context))
    }
}
impl Project {
    pub(super) fn context_for(&self, physical: Option<&Path>) -> QueryContext {
        let mut descriptor = self.context.descriptor.clone();
        if let Some(root) = physical {
            descriptor.project = root.into();
        }
        QueryContext::new(&self.runtime, descriptor)
    }
}
impl Workspace {
    pub fn project_installed(&mut self, project: Option<&Path>) {
        self.impact.project_installed(project);
        self.snapshots.project_installed(project);
        if self.project.is_none() {
            return;
        }
        let Some(project) = project else {
            self.close();
            return;
        };
        if self.identity.installed(project) == super::identity::Installation::Different {
            self.close();
        }
    }
}
