//! Filter presentation is separate from graph semantics and navigation actions.
use crate::{app::State, presentation::FilterKind, relationship::labels};
use codecallnavigator::model::{RelationshipKind, TargetResolution};
use rune::{El, row};
use rune_kit::{Checkbox, Kit};

struct FilterDescriptor {
    action: FilterKind,
    label: &'static str,
    help: &'static str,
    key: &'static str,
}

pub fn filters(state: &State, k: &Kit) -> El {
    let mut controls = row()
        .key("graph-filters")
        .padding(8.0)
        .w_full()
        .flex_wrap()
        .gap(8.0)
        .center_y()
        .radius(k.radius.sm)
        .fill(k.c.bg_base);
    let descriptors = code_filters()
        .into_iter()
        .chain(relationship_filters())
        .chain(resolution_filters());
    for descriptor in descriptors {
        controls = controls.child(
            Checkbox::new(descriptor.action.enabled(&state.nav.filter))
                .label(descriptor.label)
                .on_toggle(move |s: &mut State| s.toggle_filter(descriptor.action))
                .build(k)
                .label(descriptor.label)
                .help(descriptor.help)
                .key(descriptor.key),
        );
    }
    controls
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
