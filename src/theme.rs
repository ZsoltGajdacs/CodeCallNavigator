//! CodeGraph's shared component and graph tokens.
use rune::{Color, TextStyle, tok::Family};
use rune_kit::{Kit, PaletteSeed};

pub const SPACE: f32 = 12.0;
pub const RAIL_WIDTH: f32 = 48.0;

pub fn kit(dark: bool) -> Kit {
    let mut kit = Kit::from_palette(PaletteSeed {
        bg: hex(if dark { "#191c23" } else { "#f6f7f9" }),
        fg: hex(if dark { "#e7eaf0" } else { "#202632" }),
        accent: hex(if dark { "#69a7ff" } else { "#2164d8" }),
    });
    kit.dark = dark;
    kit.ty.ui = TextStyle::new(14.0, 400.0).lh(20.0);
    kit.ty.ui_medium = TextStyle::new(14.0, 550.0).lh(20.0);
    kit.ty.ui_compact = TextStyle::new(14.0, 400.0).lh(18.0);
    kit.radius.md = 6.0;
    kit.radius.lg = 6.0;
    kit
}

pub fn mono(size: f32, weight: f32) -> TextStyle {
    TextStyle::new(size, weight).family(Family::SystemMono)
}

pub fn canvas(k: &Kit) -> Color {
    hex(if k.dark { "#101319" } else { "#ffffff" })
}

pub fn callback(k: &Kit) -> Color {
    hex(if k.dark { "#bc9bea" } else { "#7951b0" })
}

pub fn uncertain(k: &Kit) -> Color {
    hex(if k.dark { "#dba653" } else { "#956211" })
}

pub fn hex(value: &str) -> Color {
    rune::color::hex(value).expect("CodeGraph color token")
}
