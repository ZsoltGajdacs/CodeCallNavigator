//! Serialized public queries sharing a parsed generation inside the CLI process.
use super::{query, query_executor::Executor};
use codecallnavigator_core::query::{Error, Request};
use std::io::{self, BufRead, Read, Write};

const MAX_REQUEST_BYTES: u64 = 4 * 1024 * 1024;

pub(super) fn run() -> anyhow::Result<i32> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = Executor::default();
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
