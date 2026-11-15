//! Bound pure request data before source lookup, deserialization or assembly.
use super::ExpansionRequest;
use anyhow::{Result, ensure};
use std::io::{self, Write};
pub(super) fn validate(request: &ExpansionRequest) -> Result<()> {
    request.limits().validate()?;
    ensure!(
        request.selections().len() <= 128,
        "Expansion request exceeds 128 selections"
    );
    let mut count = IdentityBytes(0);
    serde_json::to_writer(&mut count, request.root())?;
    for entry in request.selections() {
        ensure!(
            entry.caller.0.len() < 32,
            "Expansion occurrence depth exceeds 32"
        );
        for choice in entry.caller.0.iter().chain(std::iter::once(&entry.choice)) {
            choice.validate()?;
        }
        serde_json::to_writer(&mut count, &entry.caller)?;
        serde_json::to_writer(&mut count, &entry.choice)?;
    }
    Ok(())
}
struct IdentityBytes(usize);
impl Write for IdentityBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0 = self.0.saturating_add(bytes.len());
        if self.0 > 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Expansion identities exceed 1 MiB",
            ));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
