//! Exact identity union, metadata-only availability, and whole-record byte paging.
use super::{
    encoding::{self, Reply},
    endpoint::{Endpoint, Views},
    model::*,
};
use codecallnavigator_core::{
    artifact::difference::MAX_RESPONSE_BYTES,
    query::{Response, Result},
};
use std::collections::BTreeSet;

pub(super) struct CatalogRequest<'a> {
    pub text: &'a str,
    pub offset: usize,
    pub limit: usize,
}
pub(super) fn respond<'a>(
    header: Header<'a>,
    views: &Views<'a>,
    request: CatalogRequest<'a>,
    reply: &Reply<'_>,
) -> Result<Response> {
    let Views { before, after } = views;
    let ids: BTreeSet<_> = before
        .symbols()
        .chain(after.symbols())
        .filter(|symbol| matches_text(symbol, before, after, request.text))
        .collect();
    let complete =
        matches!(before, Endpoint::Available(_)) && matches!(after, Endpoint::Available(_));
    let mut page = Catalog {
        header,
        kind: "catalog",
        complete,
        endpoints: EndpointStatuses {
            before: before.status(),
            after: after.status(),
        },
        text: request.text,
        records: vec![],
        offset: request.offset,
        limit: request.limit,
        returned: request.limit,
        total: ids.len(),
        more: false,
        next_offset: Some(usize::MAX),
    };
    // Reserve the longest pagination values; each record is counted once without encoding a Vec.
    let mut used = reply.size(&page)?.ok_or_else(encoding::too_large)?;
    for symbol in ids.into_iter().skip(request.offset).take(request.limit) {
        let first = before.catalog(symbol);
        let second = after.catalog(symbol);
        let record = Record {
            symbol,
            membership: membership(&first, &second),
            before: first,
            after: second,
        };
        let remaining = MAX_RESPONSE_BYTES.saturating_sub(used).saturating_sub(1);
        let Some(size) = encoding::record_size(&record, remaining)? else {
            if page.records.is_empty() {
                page.records.push(record);
                update(&mut page);
                return reply.finish(&page);
            }
            break;
        };
        used += size + 1;
        page.records.push(record);
    }
    update(&mut page);
    reply.finish(&page)
}
fn update(page: &mut Catalog<'_>) {
    page.returned = page.records.len();
    let next = page.offset.saturating_add(page.returned);
    page.more = next < page.total;
    page.next_offset = page.more.then_some(next);
}
fn membership(before: &CatalogSide<'_>, after: &CatalogSide<'_>) -> Membership {
    match (before.membership(), after.membership()) {
        (Some(true), Some(true)) => Membership::Both,
        (Some(false), Some(true)) => Membership::Added,
        (Some(true), Some(false)) => Membership::Removed,
        _ => Membership::Unknown,
    }
}
fn matches_text(symbol: &str, before: &Endpoint<'_>, after: &Endpoint<'_>, text: &str) -> bool {
    if contains(symbol, text) {
        return true;
    }
    [before.describe(symbol), after.describe(symbol)]
        .into_iter()
        .flatten()
        .any(|node| {
            [
                node.label,
                node.signature,
                node.owner.project,
                node.owner.file,
            ]
            .into_iter()
            .any(|value| contains(value, text))
        })
}
fn contains(value: &str, text: &str) -> bool {
    text.is_empty()
        || value
            .as_bytes()
            .windows(text.len())
            .any(|value| value.eq_ignore_ascii_case(text.as_bytes()))
}
