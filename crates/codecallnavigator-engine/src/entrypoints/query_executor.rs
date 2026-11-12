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
            Ok(()) => query::respond(self.loaded.as_ref().expect("prepared generation"), request),
            Err(error) => query::envelope(None, Err(error)),
        }
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
        if let Some(loaded) = self
            .loaded
            .as_mut()
            .filter(|loaded| loaded.matches_generation(&generation, &roots))
        {
            loaded.refresh_freshness()?;
        } else {
            self.loaded = Some(query::load_bytes(request, roots, &bytes, generation)?);
        }
        Ok(())
    }
}
