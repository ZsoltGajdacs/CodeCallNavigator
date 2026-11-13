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
