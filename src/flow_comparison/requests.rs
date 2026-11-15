//! Request cancellation and wire completions retain their own publication serials.
use super::{Selection, pair};
use crate::engine_client::flow_comparison::{BoundComparison, Catalog, Error};
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
}
pub(super) struct CatalogOutput {
    pub serial: u64,
    pub physical_project: Option<std::path::PathBuf>,
    pub result: Result<CatalogAnswer, Error>,
}
pub(super) struct DetailOutput {
    pub serial: u64,
    pub binding: String,
    pub selection: Selection,
    pub result: Result<pair::Prepared, StringError>,
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
        let result = match self.operation {
            CatalogOperation::Open { project, identity } => identity
                .resolve(project.context)
                .map_err(Error::Invalid)
                .and_then(|context| {
                    physical_project = Some(context.descriptor.project.clone());
                    context
                        .flow_comparison()
                        .open(&project.base, &project.head, &self.cancel)
                        .map(|(bound, page)| {
                            CatalogAnswer::Opened(Box::new(Opened { bound, page }))
                        })
                }),
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
            physical_project,
            result,
        }
    }
}

pub(super) struct DetailWork {
    pub bound: BoundComparison,
    pub selection: Selection,
    pub dot: std::path::PathBuf,
    pub cancel: Arc<AtomicBool>,
    pub serial: u64,
}
impl DetailWork {
    pub fn run(self) -> DetailOutput {
        let binding = self.bound.header().comparison_id.clone();
        let result = self
            .bound
            .detail(
                &self.selection.symbol,
                self.selection.head_symbol.as_deref(),
                &self.cancel,
            )
            .map_err(StringError::Query)
            .and_then(|detail| {
                pair::Prepared::load(detail, &self.dot, &self.cancel)
                    .map_err(StringError::Preparation)
            });
        DetailOutput {
            serial: self.serial,
            binding,
            selection: self.selection,
            result,
        }
    }
}
