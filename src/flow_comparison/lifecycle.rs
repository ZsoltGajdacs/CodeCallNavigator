//! Capture a project once; current graph refreshes cannot silently retarget historical queries.
use super::{Project, Workspace};
use crate::{
    app::State,
    engine_client::{Descriptor, QueryContext},
};
use std::path::Path;
impl State {
    pub fn open_flow_comparison(&mut self, base: String, head: String) -> Result<(), String> {
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
            if self.root.text.trim().is_empty() {
                return Err("Choose a project folder first".into());
            }
            QueryContext::for_project(runtime, self.root.text.clone().into())
        };
        self.flow_comparison.open(Project {
            context,
            base,
            head,
            runtime: runtime.as_ref().clone(),
        });
        Ok(())
    }
}
impl Workspace {
    pub fn project_installed(&mut self, project: Option<&Path>) {
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
