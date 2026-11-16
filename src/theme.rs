//! CodeCallNavigator's shared component and graph tokens.
use rune::{Color, TextStyle, tok::Family};
use rune_kit::{Kit, PaletteSeed};

pub const SPACE: f32 = 12.0;
pub const RAIL_WIDTH: f32 = 48.0;

pub fn kit(dark: bool) -> Kit {
    let mut kit = Kit::from_palette(PaletteSeed {
        bg: hex(if dark { "#191c23" } else { "#faf9f7" }),
        fg: hex(if dark { "#e7eaf0" } else { "#20252c" }),
        accent: hex(if dark { "#69a7ff" } else { "#2164d8" }),
    });
    kit.dark = dark;
    kit.c.surface_raised = raised_surface(dark);
    kit.c.surface_raised_base = raised_surface(dark);
    kit.ty.ui = TextStyle::new(14.0, 400.0).lh(20.0);
    kit.ty.ui_medium = TextStyle::new(14.0, 550.0).lh(20.0);
    kit.ty.ui_compact = TextStyle::new(14.0, 400.0).lh(18.0);
    kit.radius.md = 8.0;
    kit.radius.lg = 12.0;
    kit
}

pub fn mono(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).family(Family::SystemMono)
}

pub fn canvas(k: &Kit) -> Color {
    hex(if k.dark { "#101319" } else { "#faf9f7" })
}

pub fn callback(k: &Kit) -> Color {
    hex(if k.dark { "#bc9bea" } else { "#7951b0" })
}

pub fn uncertain(k: &Kit) -> Color {
    hex(if k.dark { "#dba653" } else { "#956211" })
}

pub fn hex(value: &str) -> Color {
    rune::color::hex(value).expect("CodeCallNavigator color token")
}

/// Shared raised surface for the independent canvas activity and graph controls.
pub fn floating_surface(k: &Kit) -> rune::El {
    rune::col()
        .radius(12.0)
        .border(1.0, k.c.border_base)
        .fill(raised_surface(k.dark))
        .shadow(
            3.0,
            9.0,
            k.c.bg_deep.alpha(if k.dark { 0.25 } else { 0.08 }),
        )
}

/// Neutral graph cards stay visually distinct from the selected blue surface.
pub fn card_surface(k: &Kit, selected: bool, application_start: bool) -> Color {
    if selected {
        hex(if k.dark { "#1e2c40" } else { "#e5efff" })
    } else if application_start {
        k.c.text_accent.alpha(0.10)
    } else {
        hex(if k.dark { "#22262e" } else { "#f3f2ef" })
    }
}

fn raised_surface(dark: bool) -> Color {
    hex(if dark { "#22262e" } else { "#fffefd" })
}
