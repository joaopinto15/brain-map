//! The chrome: everything drawn over the graph, one object per panel.
//!
//! Each panel owns the state only it cares about — the bar owns the search text, the
//! picker owns the path being typed — and is handed the whole [`Session`] to act on. That
//! is why [`Chrome`] is a sibling of the session rather than part of it: a panel can take
//! the session mutably without borrowing the thing that owns it.

mod bar;
mod explorer;
pub mod glyph;
mod keymap;
mod legend;
mod picker;
mod settings;
mod viewport;

use crate::app::Session;
use crate::emoji::Icons;
use crate::theme::{color, Theme};
use eframe::egui::{self, Context, RichText};

#[derive(Default)]
pub struct Chrome {
    keymap: keymap::Keymap,
    bar: bar::Bar,
    hint: Hint,
    explorer: explorer::Explorer,
    legend: legend::Legend,
    settings: settings::SettingsDialog,
    picker: picker::Picker,
    viewport: viewport::Viewport,
}

impl Chrome {
    /// Order is layout: the panels claim their edges first and the graph's own region is
    /// whatever is left, which is what keeps a drag in the explorer out of the camera.
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        self.keymap.handle(ctx, session);
        // Before a vault is chosen there is nothing for any of these to be about: an
        // empty count, a legend with no rows and the keys for a graph that is not drawn.
        // The start page gets the window to itself.
        if !session.engine.graph().vault.is_empty() {
            self.bar.show(ctx, session);
            self.hint.show(ctx, session);
            self.explorer.show(ctx, session);
            self.legend.show(ctx, session);
        }
        self.settings.show(ctx, session);
        self.picker.show(ctx, session);
        self.viewport.show(ctx, session);
    }
}

/// The keys, as the window tells them to you.
#[derive(Default)]
struct Hint;

impl Hint {
    fn show(&mut self, ctx: &Context, session: &Session) {
        let theme = session.engine.ui().theme();
        egui::TopBottomPanel::bottom("hint")
            .frame(floating(theme))
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(
                        "drag or hjkl to pan · scroll to zoom · click a node to read it · \
                     / searches, n/N walk the matches · space e hides the explorer · \
                     R replays · F fullscreen · Esc releases",
                    )
                    .size(10.5)
                    .color(color(theme.muted)),
                );
            });
    }
}

/// A panel that floats over the graph rather than boxing it in.
fn floating(theme: &Theme) -> egui::Frame {
    egui::Frame::new()
        .fill(color(theme.panel))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .stroke(egui::Stroke::new(1.0_f32, color(theme.border)))
}

/// An emoji, drawn as the picture its font holds. Without a colour font it falls back to
/// the character, which is what egui would have drawn all along.
fn icon(ui: &mut egui::Ui, icons: &mut Icons, glyph: &str, side: f32) {
    match icons.texture(ui.ctx(), glyph) {
        Some(texture) => {
            let sized = egui::load::SizedTexture::from_handle(texture);
            ui.add(egui::Image::from_texture(sized).fit_to_exact_size(egui::vec2(side, side)));
        }
        None => {
            ui.label(RichText::new(glyph).size(side));
        }
    }
}
