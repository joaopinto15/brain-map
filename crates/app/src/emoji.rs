//! Colour emoji, taken out of the desktop's own font.
//!
//! egui rasterises glyphs with `ab_glyph`, which draws outlines and knows nothing about
//! colour bitmaps — which is why an emoji comes out as a grey outline however good the
//! font is. Noto Color Emoji is not outlines: every glyph is a PNG in the font's `CBDT`
//! table. So the picture is pulled out here and handed to egui as a texture, and the
//! chrome and the graph draw an image where they used to draw a character.
//!
//! `ttf-parser` and `png` do the reading, and both are already underneath `eframe`.

use eframe::egui::{ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::PathBuf;

/// Where a colour emoji font is on the systems this runs on, in the order they are
/// tried. `$BRAIN_MAP_EMOJI_FONT` comes first, which is how the Nix package points at
/// the copy it ships.
const FONTS: [&str; 5] = [
    "/usr/share/fonts/noto/NotoColorEmoji.ttf", // Arch, and Omarchy with it
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf", // Debian, Ubuntu
    "/usr/share/fonts/google-noto-emoji/NotoColorEmoji.ttf", // Fedora
    "/usr/share/fonts/NotoColorEmoji.ttf",
    "/System/Library/Fonts/Apple Color Emoji.ttc", // macOS
];

/// The strike to ask for. Noto ships one 136px bitmap, so this only has to be large
/// enough not to pick a smaller one from a font that has several.
const STRIKE: u16 = 160;

pub struct Icons {
    /// The font's bytes. The face is parsed per miss rather than held, because a parsed
    /// face borrows the bytes and this would have to borrow itself to keep one.
    font: Option<Vec<u8>>,
    /// One texture per emoji, and a remembered `None` for the ones the font has no
    /// picture of — so a miss is looked up once rather than once a frame.
    cached: HashMap<String, Option<TextureHandle>>,
}

impl Icons {
    pub fn load() -> Icons {
        let named = std::env::var_os("BRAIN_MAP_EMOJI_FONT").map(PathBuf::from);
        let font = named
            .into_iter()
            .chain(FONTS.iter().map(PathBuf::from))
            .find_map(|path| std::fs::read(path).ok());
        if font.is_none() {
            eprintln!("brain-map: no colour emoji font found — icons will be drawn as outlines");
        }
        Icons {
            font,
            cached: HashMap::new(),
        }
    }

    /// The picture of an emoji, or `None` when there is no font or no glyph — in which
    /// case the caller draws the character and gets whatever egui can make of it.
    pub fn texture(&mut self, ctx: &Context, emoji: &str) -> Option<&TextureHandle> {
        if emoji.is_empty() {
            return None;
        }
        if !self.cached.contains_key(emoji) {
            let handle = self
                .font
                .as_deref()
                .and_then(|font| image_of(font, emoji))
                .map(|image| ctx.load_texture(emoji, image, TextureOptions::LINEAR));
            self.cached.insert(emoji.to_string(), handle);
        }
        self.cached.get(emoji).and_then(Option::as_ref)
    }
}

fn image_of(font: &[u8], emoji: &str) -> Option<ColorImage> {
    let face = ttf_parser::Face::parse(font, 0).ok()?;
    // ponytail: the first code point. A joined sequence draws its base — 👨‍💻 comes out
    // as 👨 — since picking the joined glyph means walking the font's GSUB ligatures.
    let glyph = face.glyph_index(emoji.chars().next()?)?;
    let raster = face.glyph_raster_image(glyph, STRIKE)?;
    match raster.format {
        ttf_parser::RasterImageFormat::PNG => decode(raster.data),
        _ => None,
    }
}

fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Whatever the font stored, come out with four eight-bit channels.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let size = [info.width as usize, info.height as usize];
    Some(ColorImage::from_rgba_unmultiplied(
        size,
        &buffer[..info.buffer_size()],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs where a colour emoji font is installed and says so where it is not, rather
    /// than failing on a machine that simply has no emoji to read.
    #[test]
    fn an_emoji_comes_out_of_the_font_as_a_picture() {
        let Some(font) = FONTS.iter().find_map(|path| std::fs::read(path).ok()) else {
            eprintln!("no colour emoji font installed — skipped");
            return;
        };
        let penguin = image_of(&font, "\u{1f427}").expect("the penguin has a picture");
        assert!(
            penguin.width() > 16 && penguin.height() > 16,
            "{:?}",
            penguin.size
        );
        assert!(
            penguin
                .pixels
                .iter()
                .any(|p| p.a() > 0 && (p.r(), p.g(), p.b()) != (0, 0, 0)),
            "a colour bitmap, not an empty or black one"
        );
        assert!(image_of(&font, "").is_none(), "nothing is not an emoji");
        assert!(image_of(&font, "q").is_none(), "and neither is a letter");
    }
}
