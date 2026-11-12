//! Search candidates stay bounded; class members are fetched only on activation.
use super::*;
use codecallnavigator::{
    query::Category,
    search::{ClassIdentity, NameSearchQuery, SearchCategory, SearchHit, SearchPage, SearchTarget},
};

impl GraphQueries {
    pub fn search(&self, request: &NameSearchQuery<'_>, cancel: &AtomicBool) -> Result<SearchPage> {
        let Backend::Cli(context) = &self.backend else {
            return Ok(crate::fixture_queries::search(&self.graph, request));
        };
        let category = match request.category {
            SearchCategory::All => Category::All,
            SearchCategory::Classes => Category::Classes,
            SearchCategory::Methods => Category::Methods,
        };
        let result: Value = context.request(
            Query::Search {
                text: request.text.into(),
                category,
                offset: request.offset,
                limit: request.limit,
            },
            cancel,
        )?;
        let hits = result["hits"]
            .as_array()
            .context("CLI search omitted hits")?
            .iter()
            .map(|hit| self.search_hit(hit))
            .collect::<Result<_>>()?;
        Ok(SearchPage {
            hits,
            more: result["more"]
                .as_bool()
                .context("CLI search omitted pagination")?,
        })
    }

    fn search_hit(&self, hit: &Value) -> Result<SearchHit> {
        let target = &hit["target"];
        let target = if target["kind"] == "method" {
            let identity = text(target, "symbol")?;
            SearchTarget::Method {
                member: self
                    .graph
                    .node_index(&identity)
                    .context("CLI search returned a missing symbol")?,
                identity,
            }
        } else {
            ensure!(
                target["kind"] == "class",
                "CLI search returned an unsupported target kind"
            );
            SearchTarget::Class {
                identity: ClassIdentity {
                    build: text(target, "build")?,
                    project: text(target, "project")?,
                    container: text(target, "container")?,
                },
                members: Arc::from([]),
                file_container: target["file_container"]
                    .as_bool()
                    .context("CLI omitted class kind")?,
            }
        };
        Ok(SearchHit {
            name: text(hit, "name")?,
            detail: text(hit, "detail")?,
            target,
        })
    }
}
