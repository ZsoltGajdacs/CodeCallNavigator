//! Filter presentation is separate from graph semantics and navigation actions.
use crate::{
    app::State,
    presentation::{FilterKind, Overlay},
    relationship::labels,
};
use codecallnavigator::model::{RelationshipKind, TargetResolution};
use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Checkbox, Kit, Placement, Popover, popover};

struct FilterDescriptor {
    action: FilterKind,
    label: &'static str,
    help: &'static str,
    key: &'static str,
}

pub fn filters(state: &State, k: &Kit, layer: &mut Vec<El>) -> El {
    let changed = state.nav.filter != Default::default();
    let trigger = Button::new(if changed { "Filters ·" } else { "Filters" })
        .variant(ButtonVariant::Outline)
        .on_press(State::toggle_filters)
        .build(k)
        .key("graph-filters")
        .help("Choose code visibility, relationships and target resolution");
    let header = row()
        .w_full()
        .center_y()
        .justify_between()
        .child(text("Filters").font(k.ty.ui_medium).color(k.c.text_base))
        .child(
            Button::new("Reset")
                .variant(ButtonVariant::Ghost)
                .disabled(!changed)
                .on_press(State::reset_filters)
                .build(k)
                .key("reset-filters"),
        );
    let body = col()
        .w_full()
        .gap(12.0)
        .child(header)
        .child(
            text("Choose what appears in the graph.")
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(filter_section(state, k, "Code visibility", code_filters()))
        .child(filter_section(
            state,
            k,
            "Relationships",
            relationship_filters(),
        ))
        .child(filter_section(
            state,
            k,
            "Target resolution",
            resolution_filters(),
        ))
        .child(
            text("Changes apply immediately")
                .font(k.ty.small)
                .color(k.c.text_muted),
        );
    Popover::new("graph-filter-popover")
        .label("Graph filters")
        .trigger(trigger)
        .open(state.ui.overlay == Some(Overlay::Filters))
        .placement(Placement::BottomEnd)
        .on_close(State::close_overlay)
        .child(popover::fit_scroll(body))
        .build(k, layer)
}

fn filter_section<const N: usize>(
    state: &State,
    k: &Kit,
    title: &str,
    descriptors: [FilterDescriptor; N],
) -> El {
    let mut section = col()
        .w_full()
        .gap(8.0)
        .child(text(title).font(k.ty.small_medium).color(k.c.text_muted));
    for descriptor in descriptors {
        section = section.child(
            Checkbox::new(descriptor.action.enabled(&state.nav.filter))
                .label(descriptor.label)
                .on_toggle(move |s: &mut State| s.toggle_filter(descriptor.action))
                .build(k)
                .label(descriptor.label)
                .help(descriptor.help)
                .key(descriptor.key),
        );
    }
    section
}

fn code_filters() -> [FilterDescriptor; 3] {
    [
        FilterDescriptor {
            action: FilterKind::Tests,
            label: "Test code",
            help: "Show methods classified as test code.",
            key: "filter-tests",
        },
        FilterDescriptor {
            action: FilterKind::Generated,
            label: "Generated code",
            help: "Show methods classified as generated code.",
            key: "filter-generated",
        },
        FilterDescriptor {
            action: FilterKind::Unavailable,
            label: "Methods without implementation",
            help: "Show methods whose implementation is unavailable to the analyzer.",
            key: "filter-unavailable",
        },
    ]
}

fn relationship_filters() -> [FilterDescriptor; 5] {
    RelationshipKind::ALL.map(|kind| FilterDescriptor {
        action: FilterKind::Relationship(kind),
        label: labels::kind(kind),
        help: labels::kind_help(kind),
        key: match kind {
            RelationshipKind::Call => "filter-calls",
            RelationshipKind::FunctionReference => "filter-references",
            RelationshipKind::FrameworkLifecycle => "filter-framework",
            RelationshipKind::HttpNavigation => "filter-http-navigation",
            RelationshipKind::Other => "filter-other-relationships",
        },
    })
}

fn resolution_filters() -> [FilterDescriptor; 4] {
    TargetResolution::ALL.map(|resolution| FilterDescriptor {
        action: FilterKind::Resolution(resolution),
        label: labels::resolution(resolution),
        help: labels::resolution_help(resolution),
        key: match resolution {
            TargetResolution::Direct => "filter-direct",
            TargetResolution::VirtualDispatch => "filter-virtual-calls",
            TargetResolution::Possible => "filter-possible",
            TargetResolution::Other => "filter-other-resolution",
        },
    })
}
