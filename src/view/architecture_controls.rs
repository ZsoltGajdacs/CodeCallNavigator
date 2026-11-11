//! Layer setup and tab controls; file IO stays in the workspace lifecycle.
use crate::app::State;
use codecallnavigator::architecture::Selection;
use rune::{El, col, row, text};
use rune_kit::{Kit, SegmentedControl};

const PRESETS: [Selection; 4] = [
    Selection::None,
    Selection::Spring,
    Selection::React,
    Selection::Repository,
];

pub(super) fn view(state: &State, k: &Kit) -> El {
    let preset = SegmentedControl::new(
        &["None", "Spring", "React", "Repository"],
        PRESETS
            .iter()
            .position(|value| Some(*value) == state.architecture_selection),
    )
    .label("Layer configuration")
    .width(Some((state.viewport[0] - 72.0).clamp(240.0, 432.0)))
    .on_pick(|state: &mut State, index, _| state.set_architecture_selection(PRESETS[index]))
    .build(k)
    .key("layer-preset");
    let mut controls = col().w_full().padding(8.0).gap(6.0).child(
        row()
            .gap(8.0)
            .center_y()
            .child(text("Layers").font(k.ty.small).color(k.c.text_muted))
            .child(preset),
    );
    if let Some(architecture) = &state.architecture {
        controls = controls.child(
            text(&architecture.origin)
                .font(k.ty.small)
                .color(k.c.text_muted)
                .w_full()
                .wrap_anywhere(),
        );
        let mut tabs = vec![None];
        for layer in &architecture.layers {
            if layer.tab.is_some() && !tabs.contains(&layer.tab) {
                tabs.push(layer.tab.clone());
            }
        }
        if tabs.len() > 1 {
            let names: Vec<_> = tabs
                .iter()
                .map(|tab| tab.as_deref().unwrap_or("Main"))
                .collect();
            let selected = tabs.iter().position(|tab| *tab == state.nav.layer_tab);
            controls = controls.child(
                SegmentedControl::new(&names, selected)
                    .label("Architecture diagram")
                    .width(Some((state.viewport[0] - 24.0).clamp(240.0, 432.0)))
                    .on_pick(move |state: &mut State, index, _| {
                        state.set_layer_tab(tabs[index].clone())
                    })
                    .build(k)
                    .key("layer-tabs"),
            );
        }
    } else {
        controls = controls.child(text("Choose a preset or edit .codecallnavigator/layers.yml. Review configuration is imported automatically when the native file is missing.")
            .font(k.ty.small).color(k.c.text_muted).w_full().wrap());
    }
    controls
}
