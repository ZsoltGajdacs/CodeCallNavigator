//! One interactive popup shell for live graph source and saved comparison source.
use super::Interaction;
use crate::app::State;
use rune::{El, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

#[derive(Clone, Copy)]
pub enum Owner {
    Graph,
    Compare,
}

pub struct Panel<'a> {
    pub owner: Owner,
    pub bounds: [f32; 4],
    pub interaction: &'a Interaction,
}

pub struct Heading<'a> {
    pub title: &'a str,
    pub lead: Option<El>,
}

pub struct Content<'a> {
    pub heading: Heading<'a>,
    pub description: &'a str,
    pub body: El,
    pub provenance: &'a str,
}

impl Owner {
    fn key(self, name: &str) -> String {
        match self {
            Self::Graph => format!("source-{name}"),
            Self::Compare => format!("comparison-source-{name}"),
        }
    }

    fn interaction(self, state: &mut State) -> &mut Interaction {
        match self {
            Self::Graph => &mut state.source_popup.interaction,
            Self::Compare => &mut state.flow_comparison.previews.interaction,
        }
    }

    fn on_hover(self, state: &mut State, inside: bool, tx: &mut Tx) {
        self.interaction(state).popup_hover(inside, tx);
        if inside {
            match self {
                Self::Graph => state.source_popup.hover = None,
                Self::Compare => state.flow_comparison.previews.clear_hover(),
            }
        }
    }

    fn close(self, state: &mut State) {
        match self {
            Self::Graph => state.close_source(),
            Self::Compare => state.flow_comparison.previews.dismiss(),
        }
    }

    fn pin(self, state: &mut State) {
        match self {
            Self::Graph => state.pin_source(),
            Self::Compare => state.flow_comparison.previews.pin(),
        }
    }

    pub fn owns_focus(self, key: &str) -> bool {
        match self {
            Self::Graph => matches!(key, "source-pin" | "source-close" | "source-back"),
            Self::Compare => matches!(key, "comparison-source-pin" | "comparison-source-close"),
        }
    }

    pub fn focused(self, tx: &Tx) -> bool {
        tx.focus_visible() && tx.focused().is_some_and(|key| self.owns_focus(key))
    }
}

impl Panel<'_> {
    pub fn view(self, content: Content<'_>, k: &Kit) -> El {
        let Self {
            owner,
            bounds: [x, y, w, h],
            interaction,
        } = self;
        let name = match owner {
            Owner::Graph => "source-popup",
            Owner::Compare => "comparison-unified-source",
        };
        col()
            .absolute(x, y)
            .size(w, h)
            .key(name)
            .min_h(0.0)
            .fill(k.c.surface_raised)
            .border(1.0, k.c.std_border)
            .radius(12.0)
            .shadow(
                4.0,
                18.0,
                k.c.bg_deep.alpha(if k.dark { 0.35 } else { 0.12 }),
            )
            .clip_rounded(12.0)
            .on_press_tx(|_: &mut State, tx| tx.live_only())
            .role("group")
            .label("Source preview")
            .on_layout(move |s: &mut State, rect| owner.interaction(s).rect = rect)
            .on_hover_change(move |s: &mut State, inside, tx| owner.on_hover(s, inside, tx))
            .child(header(owner, interaction, content.heading, k))
            .child(
                text(content.description)
                    .key(owner.key("path"))
                    .w_full()
                    .px(12.0)
                    .font(k.ty.small)
                    .color(k.c.text_muted)
                    .truncate()
                    .help(content.description),
            )
            .child(content.body)
            .child(footer(owner, content.provenance, k))
    }
}

fn header(owner: Owner, popup: &Interaction, heading: Heading<'_>, k: &Kit) -> El {
    let Heading { title, lead } = heading;
    let mut bar = row().w_full().gap(8.0).padding(12.0).center_y();
    if let Some(lead) = lead {
        bar = bar.child(lead);
    }
    bar.child(
        text(title)
            .font(k.ty.ui_medium)
            .color(k.c.text_base)
            .min_w(0.0)
            .truncate()
            .help(title)
            .grow(1.0),
    )
    .child(
        Button::new(if popup.pinned { "Unpin" } else { "Pin" })
            .variant(ButtonVariant::Ghost)
            .on_press(move |s: &mut State| owner.pin(s))
            .build(k)
            .key(owner.key("pin")),
    )
    .child(
        Button::new("Close")
            .variant(ButtonVariant::Ghost)
            .on_press(move |s: &mut State| owner.close(s))
            .build(k)
            .key(owner.key("close")),
    )
}

fn footer(owner: Owner, provenance: &str, k: &Kit) -> El {
    row()
        .w_full()
        .h(24.0)
        .center_y()
        .child(
            text(provenance)
                .grow(1.0)
                .min_w(0.0)
                .truncate()
                .px(12.0)
                .font(k.ty.small)
                .color(k.c.text_muted),
        )
        .child(
            el().key(owner.key("resize"))
                .size(24.0, 24.0)
                .cursor(rune::Cursor::Crosshair)
                .role("button")
                .label("Resize source preview")
                .on_drag(move |s: &mut State, drag, _| owner.interaction(s).resize(drag))
                .child(text("◢").font(k.ty.small).color(k.c.text_muted)),
        )
}
