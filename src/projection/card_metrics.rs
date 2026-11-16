//! Card dimensions shared by projection, layout and native rendering.
pub(crate) const WIDTH: f32 = 248.0;
pub(crate) const PADDING: f32 = 12.0;
pub(crate) const HEADER_HEIGHT: f32 = 60.0;
pub(crate) const HEIGHT: f32 = HEADER_HEIGHT + PADDING * 2.0;
pub(crate) const MEMBER_HEIGHT: f32 = 24.0;
pub(crate) const MEMBER_GAP: f32 = 8.0;
pub(crate) const MAX_MEMBERS: usize = 12;
pub(crate) const RADIUS: f32 = 11.0;

pub(crate) fn size(members: usize) -> [f32; 2] {
    let rows = members.min(MAX_MEMBERS) as f32;
    [
        WIDTH,
        HEIGHT
            + if members == 0 {
                0.0
            } else {
                MEMBER_HEIGHT * rows + MEMBER_GAP
            },
    ]
}
