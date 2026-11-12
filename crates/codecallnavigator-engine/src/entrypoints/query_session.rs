//! Serialized public queries sharing a parsed generation inside the CLI process.
use super::query::{self, Loaded};
use codecallnavigator_core::query::{Error, Request, Response};
use std::io::{self, BufRead, Read, Write};

const MAX_REQUEST_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Default)]
struct Session {
    loaded: Option<Loaded>,
}

impl Session {
    fn respond(&mut self, request: &Request) -> Response {
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

pub(super) fn run() -> anyhow::Result<i32> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = Session::default();
    loop {
        let mut bytes = Vec::new();
        if (&mut input)
            .take(MAX_REQUEST_BYTES + 1)
            .read_until(b'\n', &mut bytes)?
            == 0
        {
            return Ok(0);
        }
        let oversized = bytes.len() as u64 > MAX_REQUEST_BYTES;
        let response = if oversized {
            query::envelope(
                None,
                Err(Error::new("invalid_request", "Query request exceeds 4 MiB")),
            )
        } else {
            match serde_json::from_slice::<Request>(&bytes) {
                Ok(request) => session.respond(&request),
                Err(error) => {
                    query::envelope(None, Err(Error::new("invalid_request", error.to_string())))
                }
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if oversized {
            return Ok(2);
        }
    }
}
