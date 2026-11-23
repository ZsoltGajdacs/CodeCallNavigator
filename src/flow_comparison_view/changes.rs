//! Outline rows are grouped by recognized route while retaining unmatched changes.
use crate::{
    app::State,
    engine_client::flow_comparison::{Change, ChangeKind, ChangeStatus},
    flow_comparison::PreviewTarget,
};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};
use std::collections::{BTreeMap, HashSet};

pub fn view(state: &State, k: &Kit) -> El {
    let workspace = &state.flow_comparison;
    let changes = &workspace.changes;
    let outline = col()
        .key("comparison-changes")
        .w_full()
        .h(changes.outline_height)
        .min_h(96.0)
        .border(1.0, k.c.std_border)
        .child(columns(["Change", "Status", "File"], 0.0, k).fill(k.c.layer_01));
    if changes.rows.is_empty() {
        return outline.child(group(empty_notice(changes), k));
    }
    outline.child(entries_view(state, k))
}

fn empty_notice(changes: &crate::flow_comparison::Changes) -> &str {
    if !changes.message.is_empty() {
        &changes.message
    } else if changes.listing() {
        "Loading changes…"
    } else {
        "No changes found"
    }
}

fn entries_view(state: &State, k: &Kit) -> El {
    let changes = &state.flow_comparison.changes;
    let grouped = Grouped::from_rows(&changes.rows);
    let outline = Outline { state, k };
    let mut list = col()
        .key("comparison-change-rows")
        .w_full()
        .min_h(0.0)
        .grow(1.0)
        .scroll(rune::ScrollSpec {
            stick_bottom: false,
        });
    if !grouped.entries.is_empty() {
        list = list.child(group("Affected entry points", k));
    }
    for ((name, id), items) in &grouped.entries {
        list = list
            .child(group(name, k).key(format!("comparison-entry-{id}")))
            .child(outline.items(items, id, 20.0));
    }
    if let Some(ungrouped) = ungrouped_view(&grouped, changes.complete, &outline) {
        list = list.child(ungrouped);
    }
    if !changes.message.is_empty() {
        list = list.child(group(&changes.message, k));
    }
    if changes.next_offset.is_some() {
        list = list.child(
            Button::new("Load more changes")
                .variant(ButtonVariant::Ghost)
                .disabled(changes.listing())
                .on_press(|s: &mut State| s.flow_comparison.next_changes())
                .build(k),
        );
    }
    list
}

fn ungrouped_view(grouped: &Grouped<'_>, complete: bool, outline: &Outline<'_>) -> Option<El> {
    let k = outline.k;
    if grouped.ungrouped.is_empty() {
        return (!complete).then(|| group("Entry point search incomplete", k));
    }
    let name = match (complete, grouped.entries.is_empty()) {
        (false, _) => "Entry point unknown · search incomplete",
        (true, true) => "Changed classes and files",
        (true, false) => "No known entry point",
    };
    Some(
        col()
            .w_full()
            .min_h(0.0)
            .child(group(name, k))
            .child(outline.items(&grouped.ungrouped, "ungrouped", 10.0)),
    )
}

type EntryGroups<'a> = BTreeMap<(&'a str, &'a str), Vec<Item<'a>>>;

#[derive(Default)]
struct Grouped<'a> {
    entries: EntryGroups<'a>,
    ungrouped: Vec<Item<'a>>,
}

impl<'a> Grouped<'a> {
    fn from_rows(rows: &'a [Change]) -> Self {
        let mut grouped = Self::default();
        for change in rows {
            if change.entry_points.is_empty() {
                grouped.ungrouped.push(Item { change, path: None });
                continue;
            }
            for entry in &change.entry_points {
                let id = (entry.name.as_str(), entry.id.as_str());
                grouped.entries.entry(id).or_default().push(Item {
                    change,
                    path: Some((entry.before.as_deref(), entry.after.as_deref())),
                });
            }
        }
        grouped
    }
}

struct Item<'a> {
    change: &'a Change,
    path: Option<(Option<&'a str>, Option<&'a str>)>,
}

struct Outline<'a> {
    state: &'a State,
    k: &'a Kit,
}

impl Outline<'_> {
    fn items(&self, items: &[Item<'_>], scope: &str, indent: f32) -> El {
        let classes: HashSet<_> = items
            .iter()
            .filter(|item| item.change.kind == ChangeKind::Class)
            .map(|item| item.change.id.as_str())
            .collect();
        let mut parents = Vec::new();
        let mut methods = BTreeMap::<&str, Vec<&Item<'_>>>::new();
        for item in items {
            match item
                .change
                .class_id
                .as_deref()
                .filter(|id| classes.contains(id))
            {
                Some(class) => methods.entry(class).or_default().push(item),
                None => parents.push(item),
            }
        }
        col()
            .w_full()
            .min_h(0.0)
            .children(parents.into_iter().flat_map(|item| {
                let parent = self.change_row(item, scope, indent);
                let children = methods
                    .get(item.change.id.as_str())
                    .into_iter()
                    .flatten()
                    .map(move |method| self.change_row(method, scope, indent + 20.0));
                std::iter::once(parent).chain(children)
            }))
    }

    fn change_row(&self, item: &Item<'_>, scope: &str, indent: f32) -> El {
        let Self { state, k } = self;
        let change = item.change;
        let label = format!("{} · {}", change.name, status(change.status));
        let key = format!("comparison-change-{scope}-{}", change.id);
        let active = match &state.flow_comparison.changes.preview {
            Some(preview) if matches!(&preview.target, PreviewTarget::Row(_)) => {
                preview.anchor == key
            }
            _ => state.flow_comparison.selected_change_id() == Some(change.id.as_str()),
        };
        let id = change.id.clone();
        let symbol = change.symbol.clone();
        let anchor = key.clone();
        let path = item.path.map(|(before, after)| match (before, after) {
            (Some("possible"), _) | (_, Some("possible")) => " · Possible path",
            (None, Some(_)) => " · After only",
            (Some(_), None) => " · Before only",
            _ => "",
        });
        let status = format!("{}{}", status(change.status), path.unwrap_or_default());
        columns([&change.name, &status, &change.file], indent, k)
            .key(key)
            .label(label)
            .role("button")
            .focusable(true)
            .fill(if active {
                k.c.text_accent.alpha(0.18)
            } else {
                k.c.bg_base
            })
            .hover(k.c.layer_01)
            .on_press(move |s: &mut State| {
                if let Some(symbol) = &symbol {
                    s.flow_comparison.choose_change_method(symbol.clone());
                } else {
                    s.flow_comparison
                        .choose_change_source(id.clone(), anchor.clone());
                }
            })
    }
}

fn columns([first, second, third]: [&str; 3], indent: f32, k: &Kit) -> El {
    row()
        .w_full()
        .h(24.0)
        .center_y()
        .px(12.0)
        .gap(8.0)
        .child(
            text(first)
                .pl(indent)
                .grow(1.6)
                .min_w(0.0)
                .font(k.ty.small)
                .color(k.c.text_base)
                .truncate(),
        )
        .child(
            text(second)
                .grow(1.0)
                .min_w(0.0)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate(),
        )
        .child(
            text(third)
                .grow(1.1)
                .min_w(0.0)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .truncate(),
        )
}

fn group(label: &str, k: &Kit) -> El {
    row()
        .w_full()
        .h(25.0)
        .px(12.0)
        .center_y()
        .fill(k.c.layer_01)
        .child(
            text(label)
                .font(k.ty.small_medium)
                .color(k.c.text_base)
                .truncate(),
        )
}

fn status(value: ChangeStatus) -> &'static str {
    match value {
        ChangeStatus::Added => "Added",
        ChangeStatus::Removed => "Removed",
        ChangeStatus::Modified => "Modified",
    }
}

pub fn divider(state: &State, k: &Kit) -> El {
    if !state.flow_comparison.changes.loaded || state.flow_comparison.changes.expert {
        return rune::el();
    }
    rune::el()
        .key("comparison-change-splitter")
        .w_full()
        .h(5.0)
        .fill(k.c.layer_01)
        .hover(k.c.text_accent.alpha(0.35))
        .cursor(rune::Cursor::ResizeRow)
        .on_drag(|state: &mut State, drag, _| {
            let height = &mut state.flow_comparison.changes.outline_height;
            if drag.phase == rune::Phase::Down {
                drag.data = *height;
            }
            *height = if drag.cancelled() {
                drag.data
            } else {
                (drag.data + drag.dy).clamp(96.0, 340.0)
            };
        })
}
