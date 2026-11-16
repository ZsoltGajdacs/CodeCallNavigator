//! Repository discovery and project explorer drawer.
use super::{explorer, pane_controls};
use crate::{app::State, theme};
use rune::{El, col};
use rune_kit::Kit;

pub fn view(state: &State, k: &Kit) -> El {
    col()
        .key("drawer")
        .w_full()
        .h_full()
        .min_h(0.0)
        .fill(k.c.bg_base)
        .child(pane_controls::header(state, k, pane_controls::Side::Files))
        .child(
            col()
                .key("drawer-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .padding(theme::SPACE)
                .child(explorer::view(state, k).w(state.ui.sidebar.width() - 2.0 * theme::SPACE)),
        )
}
