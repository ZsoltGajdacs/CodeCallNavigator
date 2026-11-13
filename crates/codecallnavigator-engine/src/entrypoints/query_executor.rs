//! Shared request dispatch and working-generation reuse for both CLI transports.
use super::query::{self, Loaded};
use codecallnavigator_core::query::{Request, Response};

#[derive(Default)]
pub(super) struct Executor {
    loaded: Option<Loaded>,
}

impl Executor {
    pub(super) fn respond(&mut self, request: &Request) -> Response {
        if let Err(error) = query::validate(request) {
            return query::envelope(None, Err(error));
        }
        if let Some(response) = super::comparison::respond(request) {
            return response;
        }
        if let Some(response) = query::live_response(request) {
            return response;
        }
        match self.prepare(request) {
            Ok(()) => self.response(request),
            Err(error) => query::envelope(None, Err(error)),
        }
    }

    fn response(&mut self, request: &Request) -> Response {
        let loaded = self.loaded.as_mut().expect("prepared generation");
        if matches!(
            request.query,
            codecallnavigator_core::query::Query::Freshness { .. }
        ) {
            return match super::observer::observe(
                &loaded.artifact,
                &loaded.generation,
                &loaded.roots,
                &super::cancellation::CANCELLED,
            ) {
                Ok(observed) => {
                    loaded.freshness = observed.legacy;
                    query::envelope(
                        Some(loaded),
                        serde_json::to_value(observed.current).map_err(|error| {
                            codecallnavigator_core::query::Error::new(
                                "serialization_failed",
                                error.to_string(),
                            )
                        }),
                    )
                }
                Err(error) => query::envelope(None, Err(error)),
            };
        }
        // Legacy queries verify inputs and hydration before attaching generation
        // context. Freshness alone deliberately avoids graph hydration/indexing.
        if let Err(error) = loaded.refresh_freshness() {
            return query::envelope(None, Err(error));
        }
        if let Err(error) = loaded.graph() {
            return query::envelope(None, Err(error));
        }
        query::respond(loaded, request)
    }

    pub(super) fn observation(
        &mut self,
        request: &Request,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> codecallnavigator_core::query::Result<super::observer::Observed> {
        self.prepare(request)?;
        let loaded = self.loaded.as_ref().expect("prepared generation");
        super::observer::observe(&loaded.artifact, &loaded.generation, &loaded.roots, cancel)
    }

    pub(super) fn refresh_plan(
        &self,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> codecallnavigator_core::query::Result<super::refresh_coordinator::Plan> {
        use codecallnavigator_core::{artifact, protocol, query::Error};
        let loaded = self
            .loaded
            .as_ref()
            .ok_or_else(|| Error::new("selection_unavailable", "No observed graph generation"))?;
        let result = (|| {
            let analysis =
                artifact::provenance::read(&loaded.artifact.graph)?.ok_or_else(|| {
                    anyhow::anyhow!(
                        "Legacy graph has no exact build selection; run a manual refresh"
                    )
                })?;
            let targets = artifact::selection::resolve(analysis.selection.as_ref(), &loaded.roots)?
                .iter()
                .map(codecallnavigator_core::discovery::rediscover_exact)
                .collect::<anyhow::Result<Vec<_>>>()?;
            let trigger = if targets.iter().any(|target| target.language == "java") {
                Some(artifact::verification::observe_refresh_trigger(
                    &loaded.artifact,
                    &loaded.generation,
                    &loaded.roots,
                    cancel,
                )?)
            } else {
                None
            };
            let project = loaded
                .roots
                .iter()
                .find(|(name, _)| name == "repo")
                .expect("loaded project root")
                .1
                .clone();
            let selection = blake3::hash(&serde_json::to_vec(&analysis.selection)?)
                .to_hex()
                .to_string();
            Ok::<_, anyhow::Error>(super::refresh_coordinator::Plan {
                request: protocol::Request::Analyze {
                    version: protocol::VERSION,
                    project: project.clone(),
                    roots: loaded.roots.clone(),
                    targets,
                    infer_linked_roots: false,
                    failure_version: Some(1),
                    expected_generation: Some(loaded.generation.clone()),
                },
                generation: loaded.generation.clone(),
                selection,
                project,
                trigger,
            })
        })();
        result.map_err(|error| Error::new("selection_unavailable", format!("{error:#}")))
    }

    fn prepare(&mut self, request: &Request) -> codecallnavigator_core::query::Result<()> {
        query::validate(request)?;
        // Always hash current saved bytes: timestamps alone cannot prove identity.
        // The parsed artifact/index is reused only for identical bytes and roots.
        let roots = query::roots(request)?;
        let project = &roots.last().expect("project root is always appended").1;
        let bytes = query::read_generation(project)?;
        let generation = blake3::hash(&bytes).to_hex().to_string();
        query::check_generation(request, &generation)?;
        if self
            .loaded
            .as_ref()
            .is_some_and(|loaded| loaded.matches_generation(&generation, &roots))
        {
            // Parsed immutable artifact/index remain executor-owned.
        } else {
            self.loaded = Some(query::load_bytes(request, roots, &bytes, generation)?);
        }
        Ok(())
    }
}
