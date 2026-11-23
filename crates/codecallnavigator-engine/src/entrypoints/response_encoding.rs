//! Count the complete serialized envelope before allocating its owned JSON representation.
use codecallnavigator_core::{
    artifact::difference::MAX_RESPONSE_BYTES,
    query::{self, Error, Response, Result},
};
use serde::Serialize;
use serde_json::Value;
use std::io::{self, Write};

pub(super) struct Reply<'a> {
    pub generation: Option<&'a str>,
    pub freshness: Option<&'a Value>,
}
#[derive(Serialize)]
struct Envelope<'a, T: Serialize> {
    version: u32,
    generation: Option<&'a str>,
    freshness: Option<&'a Value>,
    result: Option<&'a T>,
    error: Option<&'a Error>,
}
impl Reply<'_> {
    pub fn size(&self, result: &impl Serialize) -> Result<Option<usize>> {
        let envelope = Envelope {
            version: query::VERSION,
            generation: self.generation,
            freshness: self.freshness,
            result: Some(result),
            error: None,
        };
        // Both public transports append a newline to the complete response.
        count(&envelope, MAX_RESPONSE_BYTES, 1)
    }
    pub fn finish(&self, result: &impl Serialize) -> Result<Response> {
        self.finish_with(result, too_large())
    }
    pub fn finish_with(&self, result: &impl Serialize, oversized: Error) -> Result<Response> {
        if self.size(result)?.is_none() {
            return Err(oversized);
        }
        Ok(Response {
            version: query::VERSION,
            generation: self.generation.map(str::to_owned),
            freshness: self.freshness.cloned(),
            result: Some(serde_json::to_value(result).map_err(serialization)?),
            error: None,
        })
    }
}
pub(super) fn record_size(record: &impl Serialize, remaining: usize) -> Result<Option<usize>> {
    count(record, remaining, 0)
}
fn count(value: &impl Serialize, limit: usize, initial: usize) -> Result<Option<usize>> {
    let mut counter = Counter {
        size: initial,
        limit,
        exceeded: false,
    };
    match serde_json::to_writer(&mut counter, value) {
        Ok(()) => Ok(Some(counter.size)),
        Err(_) if counter.exceeded => Ok(None),
        Err(error) => Err(serialization(error)),
    }
}
struct Counter {
    size: usize,
    limit: usize,
    exceeded: bool,
}
impl Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.size) {
            self.exceeded = true;
            return Err(io::Error::other("response budget exceeded"));
        }
        self.size += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
pub(super) fn too_large() -> Error {
    Error::new(
        "response_too_large",
        "A complete flow comparison exceeds the 24 MiB response budget",
    )
}
fn serialization(error: serde_json::Error) -> Error {
    Error::new("serialization_failed", error.to_string())
}
