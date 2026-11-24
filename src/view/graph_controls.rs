//! Graph navigation controls; all graph transitions remain application actions.
use crate::{
    app::State,
    icons,
    presentation::Overlay,
    projection::{FlowDirection, Grouping},
    theme,
};
use rune::{Color, El, col, el, row, text};
use rune_kit::{
    Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Kit, SegmentedControl,
    Select,
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

#[path = "graph_filters.rs"]
mod graph_filters;
pub use graph_filters::filters;

pub fn toolbar(state: &State, k: &Kit) -> El {
    if state.nav.grouping == Grouping::Layer {
        col()
            .key("architecture-toolbar")
            .motion(rune::Spring::NONE)
            .w_full()
            .child(architecture_controls::view(state, k))
    } else {
        el()
    }
}

pub(crate) fn depth_select(key: &str, depth: u32, maximum: u32) -> Select {
    let values = (1..=maximum)
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    Select::new(key, &values)
        .label("Neighborhood depth")
        .current(Some(&depth.to_string()))
        .width(52.0)
        .disabled(maximum == 1)
}

pub fn footer(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let grouping = grouping_control(state, k, layer);
    let direction = direction_control(state, k);
    let depth_select = depth_select("depth", state.nav.depth, 64)
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
        .child(
            rune::icon(&icons::CODE_SLASH, 16.0)
                .color(k.c.text_base)
                .a11y_hidden(),
        )
        .child(grouping)
        .child(direction)
        .child(separator(k))
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
    let single_row_width = if state.has_graph_focus() && !state.nav.show_path {
        460.0
    } else {
        650.0
    };
    if state.viewport[0] < single_row_width || state.graph_view.pages > 1 {
        col()
            .key("graph-footer")
            .padding(8.0)
            .gap(6.0)
            .child(scope)
            .child(zoom)
    } else {
        row()
            .key("graph-footer")
            .padding(8.0)
            .gap(8.0)
            .center_y()
            .child(scope)
            .child(separator(k))
            .child(zoom)
    }
}

fn separator(k: &Kit) -> El {
    el().w(1.0).h(20.0).fill(k.c.border_muted).a11y_hidden()
}

fn direction_control(state: &State, k: &Kit) -> El {
    if state.has_graph_focus() && !state.nav.show_path {
        return el();
    }
    SegmentedControl::new(
        &["Callers", "Callees"],
        DIRECTIONS.iter().position(|d| *d == state.nav.direction),
    )
    .label("Relationship direction")
    .width(Some(166.0))
    .on_pick(|s: &mut State, i, _| s.set_direction(DIRECTIONS[i]))
    .build(k)
    .key("direction")
}

fn zoom(state: &State, k: &Kit) -> El {
    row()
        .gap(6.0)
        .center_y()
        .child(
            Button::new("Fit")
                .icon(&icons::OUTLINE_EXPAND)
                .variant(ButtonVariant::Ghost)
                .control_size(ControlSize::Large)
                .on_press(State::fit_graph)
                .build(k)
                .key("fit"),
        )
        .child(
            IconButton::new(&icons::MINUS)
                .label("Zoom out")
                .help("Zoom out")
                .variant(IconButtonVariant::GhostMuted)
                .control_size(ControlSize::Large)
                .on_press(|s: &mut State| s.zoom(0.8))
                .build(k)
                .key("zoom-out"),
        )
        .child(
            text(format!("{:.0}%", state.nav.camera.zoom * 100.0))
                .font(k.ty.ui)
                .color(k.c.text_muted)
                .min_w(40.0),
        )
        .child(
            IconButton::new(&icons::PLUS)
                .label("Zoom in")
                .help("Zoom in")
                .variant(IconButtonVariant::GhostMuted)
                .control_size(ControlSize::Large)
                .on_press(|s: &mut State| s.zoom(1.25))
                .build(k)
                .key("zoom-in"),
        )
}

pub fn legend(k: &Kit) -> El {
    col()
        .key("relationship-legend")
        .w_full()
        .gap(8.0)
        .child(legend_entry(
            "legend-calls",
            "Directly resolved calls",
            line_sample(k.c.text_muted, false),
            k,
        ))
        .child(legend_entry(
            "legend-other",
            "Other relationships",
            line_sample(theme::uncertain(k), true),
            k,
        ))
        .child(legend_entry(
            "legend-references",
            "Function references",
            line_sample(theme::callback(k), true),
            k,
        ))
        .child(legend_entry(
            "legend-cycle",
            "Cycle",
            el().w(40.0)
                .h(12.0)
                .radius(4.0)
                .border(1.5, theme::uncertain(k)),
            k,
        ))
}

fn legend_entry(key: &str, label: &str, sample: El, k: &Kit) -> El {
    row()
        .key(key)
        .w_full()
        .gap(12.0)
        .center_y()
        .child(el().w(44.0).h(16.0).center_y().child(sample.a11y_hidden()))
        .child(
            text(label)
                .grow(1.0)
                .min_w(0.0)
                .wrap()
                .font(k.ty.ui)
                .color(k.c.text_base),
        )
}

fn line_sample(color: Color, dashed: bool) -> El {
    if !dashed {
        return el().w(40.0).h(2.0).radius(1.0).fill(color);
    }
    row()
        .gap(4.0)
        .children((0..4).map(|_| el().w(7.0).h(2.0).radius(1.0).fill(color)))
}

fn grouping_control(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let labels = ["Project", "Namespace", "Folder", "Method", "Layer"];
    let current = GROUPS
        .iter()
        .position(|g| *g == state.nav.grouping)
        .unwrap_or(0);
    Select::new("grouping", &labels)
        .label("Group graph by")
        .current(Some(labels[current]))
        .width(108.0)
        .open(state.ui.overlay == Some(Overlay::Grouping))
        .on_toggle(|s: &mut State| {
            s.ui.overlay = if s.ui.overlay == Some(Overlay::Grouping) {
                None
            } else {
                Some(Overlay::Grouping)
            };
        })
        .on_close(State::close_overlay)
        .on_choose(|s: &mut State, value, _| {
            if let Some(index) = ["Project", "Namespace", "Folder", "Method", "Layer"]
                .iter()
                .position(|label| *label == value)
            {
                s.set_grouping(GROUPS[index]);
            }
            s.close_overlay();
        })
        .build(k, layer)
}
