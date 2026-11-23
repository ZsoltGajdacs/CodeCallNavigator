//! Request cancellation and wire completions retain their own publication serials.
use super::{ClassPair, Selection, pair};
use crate::engine_client::flow_comparison::{BoundComparison, Catalog, EndpointStatus, Error};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[derive(Default)]
pub(super) struct Clock {
    pub serial: u64,
    cancel: Arc<AtomicBool>,
}
impl Clock {
    pub fn restart(&mut self) -> Arc<AtomicBool> {
        self.cancel();
        self.cancel = Arc::new(AtomicBool::new(false));
        self.cancel.clone()
    }
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.serial += 1;
    }
}
pub(super) struct Opened {
    pub bound: BoundComparison,
    pub page: Catalog,
}
pub(super) enum CatalogAnswer {
    Opened(Box<Opened>),
    Page(Box<Catalog>),
    Refreshed {
        project: Box<super::Project>,
        opened: Box<Opened>,
    },
}
pub(super) struct CatalogOutput {
    pub serial: u64,
    pub refresh: Option<super::RefreshTicket>,
    pub physical_project: Option<std::path::PathBuf>,
    pub result: Result<CatalogAnswer, Error>,
}
pub(super) struct DetailOutput {
    pub serial: u64,
    pub binding: String,
    pub selection: DetailSelection,
    pub result: Result<PreparedDetail, StringError>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum DetailSelection {
    Function(Selection),
    Class(String),
}
pub(super) enum PreparedDetail {
    Function(Box<pair::Prepared>),
    Class(Box<ClassPair>),
}
pub(super) enum StringError {
    Query(Error),
    Preparation(String),
}
impl StringError {
    pub fn message(&self) -> String {
        match self {
            Self::Query(error) => error.to_string(),
            Self::Preparation(message) => message.clone(),
        }
    }
    pub fn mismatch(&self) -> bool {
        matches!(self,Self::Query(Error::Cli(error)) if error.code=="comparison_mismatch")
    }
}

pub(super) enum CatalogOperation {
    Open {
        project: Box<super::Project>,
        identity: super::identity::Reporter,
    },
    Refresh {
        project: Box<super::Project>,
        ticket: super::RefreshTicket,
        generation: String,
    },
    Page {
        bound: Box<BoundComparison>,
        page: crate::engine_client::flow_comparison::CatalogPage,
        physical_project: std::path::PathBuf,
    },
}
pub(super) struct CatalogWork {
    pub serial: u64,
    pub cancel: Arc<AtomicBool>,
    pub operation: CatalogOperation,
}
impl CatalogWork {
    pub fn run(self) -> CatalogOutput {
        let mut physical_project = None;
        let mut refresh = None;
        let result = match self.operation {
            CatalogOperation::Open { project, identity } => {
                let super::Project { context, refs, .. } = *project;
                identity
                    .resolve(context)
                    .map_err(Error::Invalid)
                    .and_then(|context| {
                        physical_project = Some(context.descriptor.project.clone());
                        context
                            .flow_comparison()
                            .open_selected(refs, &self.cancel)
                            .map(|(bound, page)| {
                                CatalogAnswer::Opened(Box::new(Opened { bound, page }))
                            })
                    })
            }
            CatalogOperation::Refresh {
                project,
                ticket,
                generation,
            } => {
                refresh = Some(ticket);
                physical_project = Some(project.context.descriptor.project.clone());
                project.context.flow_comparison()
                    .open_selected(project.refs.clone(), &self.cancel)
                    .and_then(|(bound, page)| {
                        if bound.header().head.digest != generation {
                            return Err(Error::Invalid(anyhow::anyhow!(
                                "The saved worktree changed again. Load to compare the latest graph."
                            )));
                        }
                        if let EndpointStatus::Unavailable(failure) = &page.endpoints.after {
                            return Err(Error::Invalid(anyhow::anyhow!("{}", failure.message)));
                        }
                        Ok(CatalogAnswer::Refreshed {
                            project,
                            opened: Box::new(Opened { bound, page }),
                        })
                    })
            }
            CatalogOperation::Page {
                bound,
                page,
                physical_project: project,
            } => {
                physical_project = Some(project);
                bound
                    .catalog(page, &self.cancel)
                    .map(|page| CatalogAnswer::Page(Box::new(page)))
            }
        };
        CatalogOutput {
            serial: self.serial,
            refresh,
            physical_project,
            result,
        }
    }
}

pub(super) struct DetailWork {
    pub bound: BoundComparison,
    pub selection: DetailSelection,
    pub dot: std::path::PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub serial: u64,
}
impl DetailWork {
    pub fn run(self) -> DetailOutput {
        let binding = self.bound.header().comparison_id.clone();
        let result = match &self.selection {
            DetailSelection::Function(selection) => self.prepare_function(selection),
            DetailSelection::Class(id) => self.prepare_class(id),
        };
        DetailOutput {
            serial: self.serial,
            binding,
            selection: self.selection,
            result,
        }
    }

    fn prepare_function(&self, selection: &Selection) -> Result<PreparedDetail, StringError> {
        let detail = self
            .bound
            .detail(
                &selection.symbol,
                selection.head_symbol.as_deref(),
                &self.cancel,
            )
            .map_err(StringError::Query)?;
        pair::Prepared::load(detail, &self.dot, &self.cancel)
            .map(|prepared| PreparedDetail::Function(Box::new(prepared)))
            .map_err(StringError::Preparation)
    }

    fn prepare_class(&self, id: &str) -> Result<PreparedDetail, StringError> {
        let graph = self
            .bound
            .class_graph(id, &self.cancel)
            .map_err(StringError::Query)?;
        ClassPair::load(graph, &self.dot, &self.cancel)
            .map(|prepared| PreparedDetail::Class(Box::new(prepared)))
            .map_err(|error| StringError::Preparation(format!("{error:#}")))
    }
}
