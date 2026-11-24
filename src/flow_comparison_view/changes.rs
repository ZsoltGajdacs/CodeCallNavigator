//! Sidebar navigation over the revision-bound changed files and symbols.
use crate::{
    app::State,
    engine_client::flow_comparison::{Change, ChangeKind, ChangeStatus},
    flow_comparison::ChangeAction,
    icons,
    projection::labels::Label,
};
use rune::{Color, El, Key, ScrollSpec, col, el, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, ScrollBar};
use std::{collections::BTreeMap, path::Path};

const SCROLL_KEY: &str = "comparison-change-rows";
const DIFF_KEY: &str = "comparison-view-source-diff";

pub fn view(state: &State, k: &Kit) -> El {
    let collapsed = state.flow_comparison.changes.sidebar.collapsed;
    let width = if collapsed { 38.0 } else { 280.0 };
    let sidebar = col()
        .key("comparison-changes")
        .w(width)
        .min_w(width)
        .h_full()
        .min_h(0.0)
        .fill(k.c.layer_01)
        .child(header(collapsed, k));
    if collapsed {
        return sidebar;
    }
    sidebar
        .child(entries_view(state, k))
        .child(diff_action(state, k))
}

fn header(collapsed: bool, k: &Kit) -> El {
    let label = if collapsed {
        "Show changes"
    } else {
        "Hide changes"
    };
    let toggle = IconButton::new(&icons::LAYOUT_LEFT)
        .label(label)
        .help(label)
        .variant(IconButtonVariant::GhostMuted)
        .expanded(!collapsed)
        .on_press_tx(|state: &mut State, tx| {
            let sidebar = &mut state.flow_comparison.changes.sidebar;
            sidebar.collapsed = !sidebar.collapsed;
            if !sidebar.collapsed {
                tx.scroll_to_y(SCROLL_KEY, sidebar.scroll.get());
            }
        })
        .build(k)
        .key("comparison-changes-toggle");
    let header = row().w_full().h(38.0).px(4.0).center_y();
    if collapsed {
        return header.child(toggle);
    }
    header
        .child(
            text("Changes")
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .pl(6.0)
                .grow(1.0),
        )
        .child(toggle)
}

fn entries_view(state: &State, k: &Kit) -> El {
    let changes = &state.flow_comparison.changes;
    let scroll = changes.sidebar.scroll.clone();
    let files = changed_files(&changes.rows);
    let outline = Outline { state, k };
    let mut list = col()
        .key(SCROLL_KEY)
        .w_full()
        .min_h(0.0)
        .grow(1.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .on_scroll_offset(move |at| {
            scroll.set(at.offset[1]);
        });
    list = list.children(
        files
            .into_iter()
            .map(|(path, file)| outline.file(path, file)),
    );
    let message = message(state);
    if !message.is_empty() {
        list = list.child(
            text(message)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .padding(10.0)
                .wrap(),
        );
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
    list.scroll_bar(k)
}

fn message(state: &State) -> &str {
    let changes = &state.flow_comparison.changes;
    if !changes.message.is_empty() {
        &changes.message
    } else if !changes.rows.is_empty() {
        ""
    } else if state.flow_comparison.revisions.base().is_none() {
        "Choose a Before revision"
    } else if changes.listing() {
        "Loading changes…"
    } else if changes.loaded {
        "No changes found"
    } else {
        ""
    }
}

fn selection(state: &State) -> Option<&str> {
    let workspace = &state.flow_comparison;
    workspace
        .changes
        .sidebar
        .selection
        .as_deref()
        .or_else(|| workspace.selected_change_id())
}

fn diff_action(state: &State, k: &Kit) -> El {
    let selected = selection(state).map(str::to_owned);
    let enabled = selected.is_some() && !state.flow_comparison.changes.reload_required;
    row().w_full().padding(8.0).child(
        Button::new("View source diff")
            .icon(&icons::CODE_SLASH)
            .variant(ButtonVariant::Ghost)
            .disabled(!enabled)
            .on_press(move |state: &mut State| {
                if let Some(id) = selected.as_ref() {
                    state
                        .flow_comparison
                        .choose_change_source(id.clone(), DIFF_KEY.into());
                }
            })
            .build(k)
            .key(DIFF_KEY),
    )
}

#[derive(Default)]
struct FileChanges<'a> {
    file: Option<&'a Change>,
    classes: BTreeMap<&'a str, ClassChanges<'a>>,
    other: Vec<&'a Change>,
}

#[derive(Default)]
struct ClassChanges<'a> {
    class: Option<&'a Change>,
    methods: Vec<&'a Change>,
}

fn changed_files(rows: &[Change]) -> BTreeMap<&str, FileChanges<'_>> {
    let mut files = BTreeMap::<&str, FileChanges<'_>>::new();
    for change in rows {
        let file = files.entry(&change.file).or_default();
        match (change.kind, change.class_id.as_deref()) {
            (ChangeKind::File, _) => file.file = Some(change),
            (ChangeKind::Class, _) => {
                file.classes.entry(&change.id).or_default().class = Some(change)
            }
            (ChangeKind::Method, Some(class)) => {
                file.classes.entry(class).or_default().methods.push(change);
            }
            (ChangeKind::Method, None) => file.other.push(change),
        }
    }
    files
}

struct Outline<'a> {
    state: &'a State,
    k: &'a Kit,
}

impl Outline<'_> {
    fn file(&self, path: &str, file: FileChanges<'_>) -> El {
        let k = self.k;
        let name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("Changed symbols");
        let key = format!("comparison-tree-file-{path}");
        let has_children = !file.classes.is_empty() || !file.other.is_empty();
        let expanded = self.state.flow_comparison.changes.sidebar.expanded(&key);
        let node = file.file.map_or_else(
            || name_and_status(name, None, k).help(path),
            |change| {
                let mut classes = file.classes.values().filter_map(|entry| entry.class);
                let only_class = classes.next().filter(|_| classes.next().is_none());
                let action = only_class.map_or(ChangeAction::File, ChangeAction::new);
                self.change_row(
                    change,
                    &Label {
                        text: name.into(),
                        description: path.into(),
                    },
                    action,
                )
            },
        );
        let mut tree = col().w_full().child(branch_row(
            node,
            has_children.then(|| branch_button(key, name, expanded, k)),
            4.0,
        ));
        if expanded {
            tree = tree
                .children(
                    file.classes
                        .into_iter()
                        .map(|(id, class)| self.class(id, class)),
                )
                .children(
                    file.other
                        .into_iter()
                        .map(|change| self.symbol_row(change, 36.0)),
                );
        }
        tree
    }

    fn class(&self, id: &str, class: ClassChanges<'_>) -> El {
        let mut tree = col().w_full();
        let mut indent = 36.0;
        let mut expanded = true;
        if let Some(change) = class.class {
            let name = symbol_label(change);
            let key = format!("comparison-tree-class-{id}");
            expanded = self.state.flow_comparison.changes.sidebar.expanded(&key);
            let toggle = (!class.methods.is_empty())
                .then(|| branch_button(key, &name.text, expanded, self.k));
            tree = tree.child(branch_row(
                self.change_row(change, &name, ChangeAction::new(change)),
                toggle,
                20.0,
            ));
            indent = 52.0;
        }
        if expanded {
            tree = tree.children(
                class
                    .methods
                    .into_iter()
                    .map(|change| self.symbol_row(change, indent)),
            );
        }
        tree
    }

    fn symbol_row(&self, change: &Change, indent: f32) -> El {
        let name = symbol_label(change);
        branch_row(
            self.change_row(change, &name, ChangeAction::new(change)),
            None,
            indent,
        )
    }

    fn change_row(&self, change: &Change, label: &Label, action: ChangeAction) -> El {
        let k = self.k;
        let name = &label.text;
        let key = format!("comparison-change-row-{}", change.id);
        let id = change.id.clone();
        let keyboard_id = id.clone();
        let keyboard = action.clone();
        name_and_status(name, Some(change.status), k)
            .key(key)
            .label(format!("{name} · {}", status(change.status)))
            .help(&label.description)
            .role("button")
            .focusable(true)
            .fill(if selection(self.state) == Some(id.as_str()) {
                k.c.text_accent.alpha(0.18)
            } else {
                k.c.layer_01
            })
            .hover(k.c.text_accent.alpha(0.1))
            .on_hover_change(|s: &mut State, inside, _| {
                if inside {
                    s.flow_comparison.previews.enter_outline();
                }
            })
            .on_press(move |s: &mut State| activate(s, &id, &action))
            .on_key(move |s: &mut State, event, _| {
                if event.key != Key::Enter {
                    return false;
                }
                activate(s, &keyboard_id, &keyboard);
                true
            })
    }
}

fn activate(state: &mut State, id: &str, action: &ChangeAction) {
    state.flow_comparison.changes.sidebar.selection = Some(id.to_owned());
    state.flow_comparison.activate_change(action);
}

fn symbol_label(change: &Change) -> Label {
    use crate::projection::labels::named;
    change.name_details.as_ref().map_or_else(
        || named(&change.name, &change.name, None),
        |details| named(&details.label, &details.description, details.visibility),
    )
}

fn branch_row(content: El, toggle: Option<El>, indent: f32) -> El {
    row()
        .w_full()
        .h(26.0)
        .center_y()
        .pl(indent)
        .pr(6.0)
        .child(toggle.unwrap_or_else(|| el().w(20.0)))
        .child(content)
}

fn branch_button(key: String, name: &str, expanded: bool, k: &Kit) -> El {
    let label = if expanded { "Collapse" } else { "Expand" };
    let icon = if expanded {
        &icons::MINUS
    } else {
        &icons::OUTLINE_CHEVRON_RIGHT
    };
    let branch = key.clone();
    IconButton::new(icon)
        .label(format!("{label} {name}"))
        .variant(IconButtonVariant::GhostMuted)
        .control_size(ControlSize::Small)
        .expanded(expanded)
        .on_press(move |s: &mut State| s.flow_comparison.changes.sidebar.toggle(&branch))
        .build(k)
        .key(key)
        .w(20.0)
}

fn name_and_status(name: &str, status_value: Option<ChangeStatus>, k: &Kit) -> El {
    let mut row = row()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .px(5.0)
        .center_y()
        .gap(6.0)
        .child(
            text(name)
                .grow(1.0)
                .min_w(0.0)
                .font(k.ty.small)
                .color(k.c.text_base)
                .truncate(),
        );
    if let Some(value) = status_value {
        row = row.child(
            text(status(value))
                .font(k.ty.small)
                .color(status_color(value, k)),
        );
    }
    row
}

fn status(value: ChangeStatus) -> &'static str {
    match value {
        ChangeStatus::Added => "Added",
        ChangeStatus::Removed => "Removed",
        ChangeStatus::Modified => "Modified",
    }
}

fn status_color(value: ChangeStatus, k: &Kit) -> Color {
    match value {
        ChangeStatus::Added => k.c.success.fg,
        ChangeStatus::Removed => k.c.text_muted,
        ChangeStatus::Modified => k.c.text_accent,
    }
}
