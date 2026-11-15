use super::{model::*, validation};
use crate::engine_client::QueryContext;
use codecallnavigator::query::{self, FlowComparisonSelection, Query};
use serde::de::DeserializeOwned;
use std::{fmt, sync::atomic::AtomicBool};

#[derive(Debug)]
pub enum Error {
    Cli(query::Error),
    Invalid(anyhow::Error),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cli(error) => write!(f, "{}: {}", error.code, error.message),
            Self::Invalid(error) => write!(f, "{error:#}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self::Invalid(error)
    }
}
pub type Result<T> = std::result::Result<T, Error>;

struct Reply<T> {
    value: T,
    generation: Option<String>,
}
impl<T> Reply<T> {
    fn check_generation(&self, header: &Header) -> Result<()> {
        if self.generation.as_deref() != Some(header.head.digest.as_str()) {
            return Err(Error::Invalid(anyhow::anyhow!(
                "CLI response generation differs from comparison head artifact"
            )));
        }
        Ok(())
    }
}
struct Binding<'a> {
    base: &'a str,
    head: &'a str,
    expected: Option<&'a str>,
}
#[derive(Clone, Debug)]
pub struct CatalogPage {
    pub text: String,
    pub offset: usize,
    pub limit: usize,
}
impl Default for CatalogPage {
    fn default() -> Self {
        Self {
            text: String::new(),
            offset: 0,
            limit: 50,
        }
    }
}
#[derive(Clone)]
pub struct ComparisonClient {
    context: QueryContext,
}
#[derive(Clone)]
pub struct BoundComparison {
    client: ComparisonClient,
    header: Header,
}
impl QueryContext {
    pub fn flow_comparison(&self) -> ComparisonClient {
        let mut context = self.clone();
        // Historical endpoint digests have no relationship to the current desktop generation.
        context.descriptor.generation = None;
        ComparisonClient { context }
    }
}
impl ComparisonClient {
    pub fn open(
        &self,
        base: &str,
        head: &str,
        cancel: &AtomicBool,
    ) -> Result<(BoundComparison, Catalog)> {
        let reply: Reply<Catalog> = self.request(
            Binding {
                base,
                head,
                expected: None,
            },
            FlowComparisonSelection::Catalog {
                text: String::new(),
                offset: 0,
                limit: 50,
            },
            cancel,
        )?;
        reply.check_generation(&reply.value.header)?;
        let page = reply.value;
        page.header.validate(base, head)?;
        validation::catalog(&page, "", 0, 50)?;
        let bound = BoundComparison {
            client: self.clone(),
            header: page.header.clone(),
        };
        Ok((bound, page))
    }
    fn request<T: DeserializeOwned>(
        &self,
        binding: Binding<'_>,
        selection: FlowComparisonSelection,
        cancel: &AtomicBool,
    ) -> Result<Reply<T>> {
        self.context.comparison_supported(cancel)?;
        let request = self.context.wire_request(Query::FlowComparison {
            base: binding.base.into(),
            head: binding.head.into(),
            expected_comparison: binding.expected.map(str::to_owned),
            selection,
        });
        let response = self.context.client.query(&request, cancel)?;
        if let Some(error) = response.error {
            return Err(Error::Cli(error));
        }
        let value = response
            .result
            .ok_or_else(|| anyhow::anyhow!("CLI omitted flow comparison result"))?;
        let value = serde_json::from_value(value).map_err(|error| {
            Error::Invalid(anyhow::anyhow!(
                "Invalid CLI flow comparison result: {error}"
            ))
        })?;
        Ok(Reply {
            value,
            generation: response.generation,
        })
    }
}
impl BoundComparison {
    pub fn header(&self) -> &Header {
        &self.header
    }
    fn check(&self, header: &Header) -> Result<()> {
        header.validate(
            &self.header.base.requested_ref,
            &self.header.head.requested_ref,
        )?;
        if header != &self.header {
            return Err(Error::Invalid(anyhow::anyhow!(
                "CLI returned another flow comparison binding"
            )));
        }
        Ok(())
    }
    pub fn catalog(&self, request: CatalogPage, cancel: &AtomicBool) -> Result<Catalog> {
        let reply: Reply<Catalog> = self.request(
            FlowComparisonSelection::Catalog {
                text: request.text.clone(),
                offset: request.offset,
                limit: request.limit,
            },
            cancel,
        )?;
        reply.check_generation(&reply.value.header)?;
        let page = reply.value;
        self.check(&page.header)?;
        validation::catalog(&page, &request.text, request.offset, request.limit)?;
        Ok(page)
    }
    pub fn detail(
        &self,
        symbol: &str,
        head_symbol: Option<&str>,
        cancel: &AtomicBool,
    ) -> Result<Detail> {
        let reply: Reply<Detail> = self.request(
            FlowComparisonSelection::Detail {
                symbol: symbol.into(),
                head_symbol: head_symbol.map(str::to_owned),
            },
            cancel,
        )?;
        reply.check_generation(&reply.value.header)?;
        let detail = reply.value;
        self.check(&detail.header)?;
        validation::detail(&detail, symbol, head_symbol.unwrap_or(symbol))?;
        Ok(detail)
    }
    fn request<T: DeserializeOwned>(
        &self,
        selection: FlowComparisonSelection,
        cancel: &AtomicBool,
    ) -> Result<Reply<T>> {
        self.client.request(
            Binding {
                base: &self.header.base.requested_ref,
                head: &self.header.head.requested_ref,
                expected: Some(&self.header.comparison_id),
            },
            selection,
            cancel,
        )
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "real_tests.rs"]
mod real_tests;
