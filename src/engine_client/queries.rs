//! Typed desktop operations over the public CLI wire contract.
use super::query_process::Client;
use crate::runtime::Runtime;
use anyhow::{Context, Result, bail, ensure};
use codecallnavigator::{
    graph::GraphIndex,
    model::GraphData,
    query::{self, Query, Response},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Descriptor {
    pub project: PathBuf,
    pub roots: Vec<(String, PathBuf)>,
    pub generation: Option<String>,
}

#[derive(Clone)]
pub struct QueryContext {
    pub descriptor: Descriptor,
    client: Arc<Client>,
}

impl QueryContext {
    pub fn flow_supported(&self, language: &str) -> bool {
        self.client.flow_supported(language)
    }
    pub fn new(runtime: &Runtime, descriptor: Descriptor) -> Self {
        Self {
            descriptor,
            client: Arc::new(Client::new(runtime.clone())),
        }
    }

    pub fn for_project(runtime: &Runtime, project: PathBuf) -> Self {
        Self::new(
            runtime,
            Descriptor {
                project,
                ..Default::default()
            },
        )
    }

    pub fn response(&self, query: Query, cancel: &AtomicBool) -> Result<Response> {
        let roots = self
            .descriptor
            .roots
            .iter()
            .filter(|(name, _)| name != "repo")
            .cloned()
            .collect();
        let request = query::Request {
            version: query::VERSION,
            project: self.descriptor.project.clone(),
            roots,
            expected_generation: self.descriptor.generation.clone(),
            query,
        };
        let response = self.client.query(&request, cancel)?;
        if let Some(error) = &response.error {
            bail!("{}: {}", error.code, error.message);
        }
        if let Some(expected) = &self.descriptor.generation {
            ensure!(
                response.generation.as_ref() == Some(expected),
                "CLI returned another graph generation"
            );
        }
        Ok(response)
    }

    pub fn request<T: DeserializeOwned>(&self, query: Query, cancel: &AtomicBool) -> Result<T> {
        let response = self.response(query, cancel)?;
        serde_json::from_value(response.result.context("CLI omitted query result")?)
            .context("Invalid CLI operation result")
    }

    pub fn snapshot(self, cancel: &AtomicBool) -> Result<Snapshot> {
        self.snapshot_with_versions(cancel, None)
    }

    pub(super) fn snapshot_with_versions(
        mut self,
        cancel: &AtomicBool,
        versions: Option<String>,
    ) -> Result<Snapshot> {
        self.descriptor.project = self
            .descriptor
            .project
            .canonicalize()
            .context("Cannot open CLI snapshot project")?;
        let response = self.response(Query::Snapshot, cancel)?;
        self.descriptor.generation = Some(
            response
                .generation
                .context("CLI snapshot omitted generation")?,
        );
        let mut data: GraphData =
            serde_json::from_value(response.result.context("CLI snapshot omitted data")?)?;
        if let Some(versions) = versions.filter(|versions| !versions.trim().is_empty()) {
            data.engine_versions = versions;
        }
        let graph = Arc::new(GraphIndex::new(Arc::new(data))?);
        let navigation = Arc::new(crate::projection::NavigationInventory::new(&graph));
        Ok(Snapshot {
            graph,
            navigation,
            context: self,
        })
    }

    pub fn observe(
        &self,
        cancel: &AtomicBool,
        receive: &dyn Fn(codecallnavigator::freshness::FreshnessObservation),
    ) -> Result<bool> {
        if !self.client.observation_supported(cancel)? {
            return Ok(false);
        }
        let mut descriptor = self.descriptor.clone();
        // Observations follow saved publications while UI adoption stays explicit.
        descriptor.generation = None;
        super::observation_process::run(
            self.client.runtime(),
            descriptor,
            cancel,
            super::ObservationOptions::passive(&|notice| {
                if let super::Notice::Observation(observation) = notice {
                    receive(observation);
                }
            }),
        )?;
        Ok(true)
    }

    pub fn observe_automatic(
        &self,
        cancel: &AtomicBool,
        controls: &std::sync::mpsc::Receiver<codecallnavigator::refresh::Control>,
        receive: &dyn Fn(super::Notice),
    ) -> Result<bool> {
        if !self.client.automatic_refresh_supported(cancel)? {
            receive(super::Notice::Activity(
                codecallnavigator::refresh::Activity::new(
                    codecallnavigator::refresh::Phase::Unavailable {
                        error: query::Error::new(
                            "automatic_refresh_unsupported",
                            "Update the CLI for automatic refresh; saved-input observation remains available",
                        ),
                    },
                ),
            ));
            return self.observe(cancel, &|observation| {
                receive(super::Notice::Observation(observation))
            });
        }
        let mut descriptor = self.descriptor.clone();
        descriptor.generation = None;
        super::observation_process::run(
            self.client.runtime(),
            descriptor,
            cancel,
            super::ObservationOptions {
                automatic: true,
                receive,
                controls: Some(controls),
            },
        )?;
        Ok(true)
    }
}

pub struct Snapshot {
    pub navigation: Arc<crate::projection::NavigationInventory>,
    pub graph: Arc<GraphIndex>,
    pub context: QueryContext,
}
