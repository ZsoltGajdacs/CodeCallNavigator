//! Shared pane affordances: presentation state never changes graph navigation.
use crate::{app::State, icons, presentation::Drawer, theme};
use rune::{El, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit};

#[derive(Clone, Copy)]
pub(crate) enum Side {
    Files,
    Inspector,
}

impl Side {
    fn title(self) -> &'static str {
        match self {
            Self::Files => "Files",
            Self::Inspector => "Inspector",
        }
    }

    fn open(self, state: &State) -> bool {
        match self {
            Self::Files => state.ui.sidebar.is_open(),
            Self::Inspector => state.ui.inspector.is_open(),
        }
    }

    fn toggle(self, state: &mut State) {
        match self {
            Self::Files => state.toggle_drawer(Drawer::Repository),
            Self::Inspector => state.toggle_inspector(),
        }
    }
}

fn button(state: &State, k: &Kit, side: Side, close: bool) -> El {
    let icon = match side {
        Side::Files => &icons::LAYOUT_LEFT,
        Side::Inspector => &icons::LAYOUT_RIGHT,
    };
    let key = match (side, close) {
        (Side::Files, false) => "drawer-repository",
        (Side::Files, true) => "close-files",
        (Side::Inspector, false) => "toggle-inspector",
        (Side::Inspector, true) => "close-inspector",
    };
    IconButton::new(icon)
        .label(if close {
            format!("Close {}", side.title())
        } else {
            side.title().into()
        })
        .help(format!(
            "{} {}",
            if side.open(state) { "Hide" } else { "Show" },
            side.title()
        ))
        .variant(IconButtonVariant::GhostMuted)
        .expanded(side.open(state))
        .on_press(move |s: &mut State| side.toggle(s))
        .build(k)
        .key(key)
}

pub(super) fn toggle(state: &State, k: &Kit, side: Side) -> El {
    button(state, k, side, false)
}

pub(crate) fn header(state: &State, k: &Kit, side: Side) -> El {
    row()
        .w_full()
        .h(44.0)
        .px(theme::SPACE)
        .center_y()
        .child(
            text(side.title())
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .grow(1.0),
        )
        .child(button(state, k, side, true))
}
