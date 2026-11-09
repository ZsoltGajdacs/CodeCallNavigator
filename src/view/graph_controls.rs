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

const GROUPS: [Grouping; 4] = [
    Grouping::Project,
    Grouping::Namespace,
    Grouping::Folder,
    Grouping::Method,
];
const DIRECTIONS: [FlowDirection; 2] = [FlowDirection::Callers, FlowDirection::Callees];

pub fn filters(state: &State, k: &Kit) -> El {
    let f = &state.nav.filter;
    let compact = state.viewport[0] < 930.0;
    let mut controls = row()
        .key("graph-filters")
        .absolute(8.0, 8.0)
        .padding(8.0)
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
                .help(label)
                .key(key),
        );
    }
    controls
}

pub fn toolbar(state: &State, k: &Kit) -> El {
    let navigation = row()
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
        );
    let grouping = SegmentedControl::new(
        &["Project", "Namespace", "Folder", "Method"],
        GROUPS.iter().position(|g| *g == state.nav.grouping),
    )
    .label("Group graph by")
    .width(Some(432.0))
    .on_pick(|s: &mut State, i, _| s.set_grouping(GROUPS[i]))
    .build(k)
    .key("grouping");
    if state.viewport[0] < 760.0 {
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
    }
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
            text("Solid · direct     Amber dashes · uncertain")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            text("Purple dashes · callback     Amber outline · cycle")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
}
