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
    freshness: Option<codecallnavigator::artifact::verification::InputStatus>,
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
    base_snapshot: Option<&'a str>,
    head_snapshot: Option<&'a str>,
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
    references: super::ComparisonRefs,
    pub(crate) freshness: Option<codecallnavigator::artifact::verification::InputStatus>,
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
    #[cfg(test)]
    pub fn open(
        &self,
        base: &str,
        head: &str,
        cancel: &AtomicBool,
    ) -> Result<(BoundComparison, Catalog)> {
        self.open_selected(
            super::ComparisonRefs {
                base: base.into(),
                head: head.into(),
                ..Default::default()
            },
            cancel,
        )
    }
    pub fn open_selected(
        &self,
        references: super::ComparisonRefs,
        cancel: &AtomicBool,
    ) -> Result<(BoundComparison, Catalog)> {
        let reply: Reply<Catalog> = self.request(
            Binding {
                base: &references.base,
                head: &references.head,
                base_snapshot: references.base_snapshot.as_deref(),
                head_snapshot: references.head_snapshot.as_deref(),
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
        references.validate(&page.header)?;
        validation::catalog(&page, "", 0, 50)?;
        let bound = BoundComparison {
            client: self.clone(),
            header: page.header.clone(),
            references,
            freshness: reply.freshness,
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
        if binding.base_snapshot.is_some() || binding.head_snapshot.is_some() {
            self.context.require_local_snapshots(cancel)?;
        }
        let request = self.context.wire_request(Query::FlowComparison {
            base: binding.base.into(),
            head: binding.head.into(),
            base_snapshot: binding.base_snapshot.map(str::to_owned),
            head_snapshot: binding.head_snapshot.map(str::to_owned),
            expected_comparison: binding.expected.map(str::to_owned),
            selection,
        });
        self.reply(request, cancel)
    }
    fn reply<T: DeserializeOwned>(
        &self,
        request: query::Request,
        cancel: &AtomicBool,
    ) -> Result<Reply<T>> {
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
            freshness: worktree_freshness(response.freshness),
        })
    }
}
impl BoundComparison {
    pub(super) fn changes_capability(&self, cancel: &AtomicBool) -> anyhow::Result<bool> {
        self.client
            .context
            .client
            .comparison_changes_supported(cancel)
    }
    pub(super) fn change_request<T: DeserializeOwned>(
        &self,
        selection: query::ComparisonChangesSelection,
        cancel: &AtomicBool,
        header: impl FnOnce(&T) -> &Header,
    ) -> Result<T> {
        let request = self.client.context.wire_request(Query::ComparisonChanges {
            base: self.references.base.clone(),
            head: self.references.head.clone(),
            base_snapshot: self.references.base_snapshot.clone(),
            head_snapshot: self.references.head_snapshot.clone(),
            expected_comparison: Some(self.header.comparison_id.clone()),
            selection,
        });
        let reply = self.client.reply(request, cancel)?;
        let actual = header(&reply.value);
        reply.check_generation(actual)?;
        self.check(actual)?;
        Ok(reply.value)
    }
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
                base_snapshot: self.references.base_snapshot.as_deref(),
                head_snapshot: self.references.head_snapshot.as_deref(),
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
