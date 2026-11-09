//! Graph navigation controls; all graph transitions remain application actions.
use crate::{
    app::State,
    icons,
    presentation::{FilterKind, Overlay},
    projection::{FlowDirection, Grouping},
    theme,
};
use rune::{El, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, Checkbox, IconButton, IconButtonVariant, Kit, SegmentedControl, Select,
};

#[path = "architecture_controls.rs"]
mod architecture_controls;

const GROUPS: [Grouping; 5] = [
    Grouping::Project,
    Grouping::Namespace,
    Grouping::Folder,
    Grouping::Method,
    Grouping::Layer,
];
const DIRECTIONS: [FlowDirection; 2] = [FlowDirection::Callers, FlowDirection::Callees];

fn filter_help(kind: FilterKind, label: &str) -> &str {
    match kind {
        FilterKind::VirtualCalls => {
            "Calls to declared virtual or interface methods. An override may execute at runtime."
        }
        _ => label,
    }
}

pub fn filters(state: &State, k: &Kit) -> El {
    let f = &state.nav.filter;
    let compact = state.viewport[0] < 1150.0;
    let mut controls = row()
        .key("graph-filters")
        .absolute(8.0, 8.0)
        .padding(8.0)
        .max_w((state.viewport[0] - 16.0).max(0.0))
        .flex_wrap()
        .gap(8.0)
        .center_y()
        .radius(k.radius.sm)
        .fill(k.c.bg_base);
    for (label, short, key, checked, kind) in [
        (
            "Test code",
            "Tests",
            "filter-tests",
            f.include_tests,
            FilterKind::Tests,
        ),
        (
            "Generated code",
            "Gen",
            "filter-generated",
            f.include_generated,
            FilterKind::Generated,
        ),
        (
            "Unavailable bodies",
            "No body",
            "filter-unavailable",
            f.include_unavailable,
            FilterKind::Unavailable,
        ),
        (
            "Virtual / interface calls",
            "Virtual calls",
            "filter-virtual-calls",
            f.include_virtual_calls,
            FilterKind::VirtualCalls,
        ),
        (
            "Possible / uncertain targets",
            "Uncertain",
            "filter-uncertain",
            f.include_uncertain,
            FilterKind::Uncertain,
        ),
        (
            "Callbacks / references",
            "Refs",
            "filter-references",
            f.include_references,
            FilterKind::References,
        ),
    ] {
        controls = controls.child(
            Checkbox::new(checked)
                .label(if compact { short } else { label })
                .on_toggle(move |s: &mut State| s.toggle_filter(kind))
                .build(k)
                .label(label)
                .help(filter_help(kind, label))
                .key(key),
        );
    }
    controls
}

pub fn toolbar(state: &State, k: &Kit) -> El {
    let navigation = navigation_controls(state, k);
    let grouping = SegmentedControl::new(
        &["Project", "Namespace", "Folder", "Method", "Layer"],
        GROUPS.iter().position(|g| *g == state.nav.grouping),
    )
    .label("Group graph by")
    .width(Some((state.viewport[0] - 16.0).min(492.0)))
    .on_pick(|s: &mut State, i, _| s.set_grouping(GROUPS[i]))
    .build(k)
    .key("grouping");
    let toolbar = if state.viewport[0] < 820.0 {
        col()
            .key("graph-toolbar")
            .w_full()
            .padding(8.0)
            .gap(6.0)
            .child(navigation)
            .child(grouping)
    } else {
        row()
            .key("graph-toolbar")
            .w_full()
            .padding(8.0)
            .gap(12.0)
            .center_y()
            .child(navigation)
            .child(grouping)
    };
    if state.nav.grouping == Grouping::Layer {
        col()
            .key("architecture-toolbar")
            .motion(rune::Spring::NONE)
            .w_full()
            .child(toolbar)
            .child(architecture_controls::view(state, k))
    } else {
        toolbar
    }
}

fn navigation_controls(state: &State, k: &Kit) -> El {
    row()
        .gap(4.0)
        .center_y()
        .child(
            IconButton::new(&icons::OUTLINE_ARROW_LEFT)
                .label("Back")
                .help("Back")
                .variant(IconButtonVariant::GhostMuted)
                .disabled(state.back.is_empty())
                .on_press(|s: &mut State| s.history(false))
                .build(k)
                .key("back"),
        )
        .child(
            IconButton::new(&icons::OUTLINE_ARROW_RIGHT)
                .label("Forward")
                .help("Forward")
                .variant(IconButtonVariant::GhostMuted)
                .disabled(state.forward.is_empty())
                .on_press(|s: &mut State| s.history(true))
                .build(k)
                .key("forward"),
        )
        .child(
            Button::new("Overview")
                .variant(ButtonVariant::Ghost)
                .on_press(State::overview)
                .build(k)
                .key("overview"),
        )
        .child(
            Button::new("Collapse")
                .variant(ButtonVariant::Ghost)
                .on_press(State::collapse_all)
                .build(k)
                .key("collapse-all"),
        )
}

pub fn footer(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let direction = SegmentedControl::new(
        &["Callers", "Callees"],
        DIRECTIONS.iter().position(|d| *d == state.nav.direction),
    )
    .label("Relationship direction")
    .width(Some(166.0))
    .on_pick(|s: &mut State, i, _| s.set_direction(DIRECTIONS[i]))
    .build(k)
    .key("direction");
    let depth = state.nav.depth.to_string();
    let values = (1..=64).map(|v| v.to_string()).collect::<Vec<_>>();
    let depth_select = Select::new("depth", &values)
        .label("Neighborhood depth")
        .current(Some(&depth))
        .width(100.0)
        .open(state.ui.overlay == Some(Overlay::Depth))
        .on_toggle(State::toggle_depth)
        .on_close(State::close_overlay)
        .on_choose(|s: &mut State, v, _| {
            if let Ok(depth) = v.parse() {
                s.set_depth(depth);
            }
            s.close_overlay();
        })
        .build(k, layer);
    let scope = row()
        .gap(8.0)
        .center_y()
        .child(direction)
        .child(text("Depth").font(k.ty.small).color(k.c.text_muted))
        .child(depth_select);
    let mut zoom = zoom(state, k);
    if state.graph_view.pages > 1 {
        zoom = zoom.child(
            Button::new(format!(
                "Page {}/{}",
                state.nav.page + 1,
                state.graph_view.pages
            ))
            .help("Next graph page")
            .on_press(State::next_graph_page)
            .build(k)
            .key("next-page"),
        );
    }
    if state.viewport[0] < 650.0 || state.graph_view.pages > 1 {
        col()
            .key("graph-footer")
            .w_full()
            .padding(8.0)
            .gap(6.0)
            .child(scope)
            .child(zoom)
    } else {
        row()
            .key("graph-footer")
            .w_full()
            .padding(8.0)
            .gap(8.0)
            .center_y()
            .child(scope)
            .child(el().grow(1.0))
            .child(zoom)
    }
}

fn zoom(state: &State, k: &Kit) -> El {
    row()
        .gap(4.0)
        .center_y()
        .child(
            IconButton::new(&icons::MINUS)
                .label("Zoom out")
                .help("Zoom out")
                .on_press(|s: &mut State| s.zoom(0.8))
                .build(k)
                .key("zoom-out"),
        )
        .child(
            Button::new("Fit")
                .on_press(State::fit_graph)
                .build(k)
                .key("fit"),
        )
        .child(
            IconButton::new(&icons::PLUS)
                .label("Zoom in")
                .help("Zoom in")
                .on_press(|s: &mut State| s.zoom(1.25))
                .build(k)
                .key("zoom-in"),
        )
        .child(
            text(format!("{:.0}%", state.nav.camera.zoom * 100.0))
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}

pub fn legend(k: &Kit) -> El {
    col()
        .key("relationship-legend")
        .w_full()
        .px(theme::SPACE)
        .py(6.0)
        .gap(2.0)
        .child(
            text("Solid · direct     Amber dashes · virtual/interface or uncertain")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            text("Purple dashes · callback     Amber outline · cycle")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}
