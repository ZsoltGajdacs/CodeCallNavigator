//! Match changed lines to method spans in independently verified saved source.
use super::{normalize, texts};
use crate::{
    artifact::{
        comparison_changes::{
            Change, ChangeKind, Facet, NameDetails, Status,
            class::Class,
            display_signature,
            reachability::{self, Search},
            snapshot::{FileId, Snapshot, is_method},
            source, stable_id,
        },
        revision::SourceFile,
    },
    model::{Node, Visibility},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Attribution<'s, 'g> {
    before: Option<IndexedSide<'s, 'g>>,
    after: Option<IndexedSide<'s, 'g>>,
}

impl<'s, 'g> Attribution<'s, 'g> {
    pub fn new(before: Option<&'s Snapshot<'g>>, after: Option<&'s Snapshot<'g>>) -> Self {
        Self {
            before: before.map(IndexedSide::new),
            after: after.map(IndexedSide::new),
        }
    }

    pub fn complete(&self) -> bool {
        [&self.before, &self.after]
            .into_iter()
            .all(|side| side.as_ref().is_some_and(|side| side.search.complete))
    }

    pub fn rows(&mut self, file: &SourceFile) -> Vec<Change> {
        let Ok((old, new)) = texts(file) else {
            return Vec::new();
        };
        if old == new {
            return Vec::new();
        }
        let edit = VerifiedEdit {
            before: FileEvidence::new(
                self.before.as_ref(),
                &file.path,
                &old,
                file.before.present(),
            ),
            after: FileEvidence::new(self.after.as_ref(), &file.path, &new, file.after.present()),
            graph_pair: self.before.is_some() && self.after.is_some(),
        };
        if edit.before.matched.is_none() && edit.after.matched.is_none() {
            return Vec::new();
        }
        let patch = source::patch(&old, &new);
        let symbols = edit.changed_symbols(&patch);
        let mut classes = BTreeSet::new();
        let mut rows = Vec::new();
        for symbol in symbols {
            let (method, class) = self.method_row(&file.path, &edit, symbol);
            classes.extend(class);
            rows.push(method);
        }
        rows.extend(
            classes
                .iter()
                .map(|class| self.class_row(&file.path, &edit, class)),
        );
        rows
    }

    fn method_row(
        &mut self,
        path: &str,
        edit: &VerifiedEdit<'_, '_>,
        symbol: &str,
    ) -> (Change, Option<Class>) {
        let old = edit.before.member(symbol);
        let new = edit.after.member(symbol);
        let node = new
            .or(old)
            .expect("a changed symbol has verified source")
            .node;
        let class = Class::from_node(node).filter(|class| {
            [&self.before, &self.after]
                .into_iter()
                .flatten()
                .all(|side| side.class_in_file(class, path))
        });
        let row = Change {
            id: stable_id("method", &[symbol]),
            kind: ChangeKind::Method,
            name: display_signature(node),
            name_details: Some(NameDetails::method(node)),
            file: path.to_owned(),
            status: edit.status(old.is_some(), new.is_some()),
            changes_in: vec![Facet::Source],
            entry_points: self.routes(
                old.map(|member| member.index),
                new.map(|member| member.index),
            ),
            class_id: class.as_ref().map(Class::id),
            symbol: Some(symbol.into()),
            flow_available: Some(edit.can_pair() && edit.has_flow(symbol)),
        };
        (row, class)
    }

    fn class_row(&mut self, path: &str, edit: &VerifiedEdit<'_, '_>, class: &Class) -> Change {
        let old = edit.before.class_members(class);
        let new = edit.after.class_members(class);
        let visibility = if new.is_empty() {
            edit.before.class_visibility(class, &old)
        } else {
            edit.after.class_visibility(class, &new)
        };
        Change {
            id: class.id(),
            kind: ChangeKind::Class,
            name: class.name().to_owned(),
            name_details: Some(class.name_details(visibility)),
            file: path.to_owned(),
            status: edit.status(!old.is_empty(), !new.is_empty()),
            changes_in: vec![Facet::Members],
            entry_points: self.routes(old, new),
            class_id: None,
            symbol: None,
            flow_available: Some(edit.can_pair()),
        }
    }

    fn routes(
        &mut self,
        before_members: impl IntoIterator<Item = usize>,
        after_members: impl IntoIterator<Item = usize>,
    ) -> Vec<crate::artifact::comparison_changes::EntryPoint> {
        reachability::routes(
            self.before.as_mut().map(|side| &mut side.search),
            before_members,
            self.after.as_mut().map(|side| &mut side.search),
            after_members,
        )
    }
}

struct VerifiedEdit<'s, 'g> {
    before: FileEvidence<'s, 'g>,
    after: FileEvidence<'s, 'g>,
    graph_pair: bool,
}

impl<'g> VerifiedEdit<'_, 'g> {
    fn changed_symbols(&self, patch: &diffy::Patch<'_, str>) -> BTreeSet<&'g str> {
        self.before
            .changed_symbols(source::changed_lines(patch, true))
            .into_iter()
            .chain(
                self.after
                    .changed_symbols(source::changed_lines(patch, false)),
            )
            .collect()
    }

    fn confirmed(&self) -> bool {
        self.before.confirmed() && self.after.confirmed()
    }

    fn can_pair(&self) -> bool {
        self.graph_pair && self.confirmed()
    }

    fn has_flow(&self, symbol: &str) -> bool {
        self.before.has_flow(symbol) || self.after.has_flow(symbol)
    }

    fn status(&self, old: bool, new: bool) -> Status {
        if self.confirmed() {
            Status::between(old, new)
        } else {
            Status::Modified
        }
    }
}

struct IndexedSide<'s, 'g> {
    snapshot: &'s Snapshot<'g>,
    files: BTreeMap<&'g str, Vec<FileId<'g>>>,
    search: Search<'g>,
}

impl<'s, 'g> IndexedSide<'s, 'g> {
    fn new(snapshot: &'s Snapshot<'g>) -> Self {
        let mut files = BTreeMap::<_, Vec<_>>::new();
        for &file in snapshot.files.keys() {
            if let Some(path) = file.2.strip_prefix("@repo/") {
                files.entry(path).or_default().push(file);
            }
        }
        Self {
            snapshot,
            files,
            search: Search::new(snapshot.graph),
        }
    }

    fn match_file(&self, path: &str, contents: &str) -> Option<VerifiedFile<'s, 'g>> {
        let files = self.files.get(path)?;
        let mut matching = files.iter().filter_map(|&file| {
            let saved = self.snapshot.text(file)?;
            (normalize(saved) == contents).then_some(VerifiedFile {
                snapshot: self.snapshot,
                file,
                saved,
            })
        });
        let matched = matching.next()?;
        matching.next().is_none().then_some(matched)
    }

    fn class_in_file(&self, class: &Class, path: &str) -> bool {
        self.snapshot
            .classes
            .get(class)
            .into_iter()
            .flatten()
            .all(|&index| {
                self.snapshot.graph.nodes[index].file.strip_prefix("@repo/") == Some(path)
            })
    }
}

#[derive(Clone, Copy)]
struct Member<'g> {
    index: usize,
    node: &'g Node,
}

struct VerifiedFile<'s, 'g> {
    snapshot: &'s Snapshot<'g>,
    file: FileId<'g>,
    saved: &'g str,
}

struct FileEvidence<'s, 'g> {
    exists: bool,
    matched: Option<VerifiedFile<'s, 'g>>,
}

impl<'s, 'g> FileEvidence<'s, 'g> {
    fn new(side: Option<&IndexedSide<'s, 'g>>, path: &str, contents: &str, exists: bool) -> Self {
        let matched = if exists {
            side.and_then(|side| side.match_file(path, contents))
        } else {
            None
        };
        Self { exists, matched }
    }

    fn confirmed(&self) -> bool {
        !self.exists || self.matched.is_some()
    }

    fn member(&self, symbol: &str) -> Option<Member<'g>> {
        let file = self.matched.as_ref()?;
        let index = *file.snapshot.symbols.get(symbol)?;
        let node = &file.snapshot.graph.nodes[index];
        (FileId::of_node(node) == file.file && is_method(node)).then_some(Member { index, node })
    }

    fn has_flow(&self, symbol: &str) -> bool {
        self.matched
            .as_ref()
            .is_some_and(|file| file.snapshot.flow_source(symbol).is_some())
    }

    fn class_members(&self, class: &Class) -> Vec<usize> {
        self.matched.as_ref().map_or_else(Vec::new, |file| {
            file.snapshot
                .classes
                .get(class)
                .into_iter()
                .flatten()
                .copied()
                .filter(|&index| FileId::of_node(&file.snapshot.graph.nodes[index]) == file.file)
                .collect()
        })
    }

    fn class_visibility(&self, class: &Class, members: &[usize]) -> Option<Visibility> {
        class.visibility(self.matched.as_ref()?.snapshot.graph, members)
    }

    fn changed_symbols(&self, lines: impl Iterator<Item = usize>) -> BTreeSet<&'g str> {
        self.matched
            .as_ref()
            .map_or_else(BTreeSet::new, |file| file.changed_symbols(lines))
    }
}

impl<'g> VerifiedFile<'_, 'g> {
    fn changed_symbols(&self, lines: impl Iterator<Item = usize>) -> BTreeSet<&'g str> {
        let mut owners = lines
            .map(|line| (line, Ownership::None))
            .collect::<BTreeMap<_, _>>();
        if owners.is_empty() {
            return BTreeSet::new();
        }
        let newlines = self
            .saved
            .bytes()
            .enumerate()
            .filter_map(|(at, byte)| (byte == b'\n').then_some(at))
            .collect::<Vec<_>>();
        for (symbol, range) in self.method_spans(&newlines) {
            for (_, owner) in owners.range_mut(range) {
                owner.record(symbol);
            }
        }
        owners
            .into_values()
            .filter_map(|owner| match owner {
                Ownership::Unique(symbol) => Some(symbol),
                _ => None,
            })
            .collect()
    }

    fn method_spans<'a>(
        &'a self,
        newlines: &'a [usize],
    ) -> impl Iterator<Item = (&'g str, std::ops::RangeInclusive<usize>)> + 'a {
        self.snapshot
            .file_nodes
            .get(&self.file)
            .into_iter()
            .flatten()
            .filter_map(move |&index| {
                let node = &self.snapshot.graph.nodes[index];
                if !is_method(node) {
                    return None;
                }
                let (_, span) = self.snapshot.flow_source(&node.id)?;
                let start = newlines.partition_point(|&at| at < span.start) + 1;
                let end = newlines.partition_point(|&at| at < span.end - 1) + 1;
                Some((node.id.as_str(), start..=end))
            })
    }
}

enum Ownership<'a> {
    None,
    Unique(&'a str),
    Ambiguous,
}

impl<'a> Ownership<'a> {
    fn record(&mut self, symbol: &'a str) {
        *self = match self {
            Self::None => Self::Unique(symbol),
            _ => Self::Ambiguous,
        };
    }
}
