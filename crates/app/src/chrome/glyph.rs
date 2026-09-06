//! Every character the chrome draws as a control, in one place.
//!
//! egui draws text from four bundled fonts and nothing else. A character in none of them
//! comes out as an empty box, with no warning and no error — which is exactly what `✎`
//! (U+270E, in no font at all) and `✕` (U+2715, likewise) did to the edit and close
//! buttons. Picking a character that *looks* right is not enough, so they are named here
//! and the test below checks each one against the fonts that will have to draw it.

pub const RESCAN: &str = "⟳";
pub const SETTINGS: &str = "⚙";
pub const FULLSCREEN: &str = "⛶";
/// U+270F, not U+270E: the lower-right pencil is a dingbat and is not bundled.
pub const EDIT: &str = "✏";
/// U+2716, not U+2715: the plain multiplication x is a dingbat and is not bundled.
pub const CLOSE: &str = "✖";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::{BULLET, TASK_DONE, TASK_TODO};

    /// The fonts egui bundles and draws from, in the order it falls back through them.
    const FONTS: [(&str, &[u8]); 4] = [
        ("Ubuntu-Light", epaint_default_fonts::UBUNTU_LIGHT),
        (
            "NotoEmoji-Regular",
            epaint_default_fonts::NOTO_EMOJI_REGULAR,
        ),
        ("emoji-icon-font", epaint_default_fonts::EMOJI_ICON),
        ("Hack-Regular", epaint_default_fonts::HACK_REGULAR),
    ];

    /// Every character the window draws itself — the chrome's controls and the markers the
    /// note renderer emits — has to be in a font egui will actually reach for. A glyph
    /// that is in none of them is invisible rather than wrong, so nothing else catches it.
    #[test]
    fn every_glyph_the_window_draws_is_in_a_font_that_can_draw_it() {
        let named = [
            ("rescan", RESCAN),
            ("settings", SETTINGS),
            ("fullscreen", FULLSCREEN),
            ("edit", EDIT),
            ("close", CLOSE),
            ("done task", TASK_DONE),
            ("todo task", TASK_TODO),
            ("bullet", BULLET),
        ];
        let faces: Vec<(&str, ttf_parser::Face)> = FONTS
            .iter()
            .map(|(name, bytes)| (*name, ttf_parser::Face::parse(bytes, 0).expect("a font")))
            .collect();

        let missing: Vec<String> = named
            .iter()
            .filter(|(_, glyph)| {
                let c = glyph.chars().next().expect("a character");
                !faces.iter().any(|(_, face)| face.glyph_index(c).is_some())
            })
            .map(|(what, glyph)| {
                let c = glyph.chars().next().expect("a character");
                format!(
                    "{what} is {glyph} (U+{:04X}), which no bundled font has",
                    c as u32
                )
            })
            .collect();
        assert!(missing.is_empty(), "{}", missing.join("; "));
    }
}
