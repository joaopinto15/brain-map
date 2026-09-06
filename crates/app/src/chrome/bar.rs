//! The bar: what you do, rather than what you have set. The count, the search box and the
//! two buttons — everything configurable lives in the settings dialog instead.

use super::{floating, glyph};
use crate::app::Session;
use crate::theme::color;
use eframe::egui::{self, Context, Id, Key, RichText};

#[derive(Default)]
pub struct Bar {
    /// The search box's text belongs to the search box.
    query: String,
}

impl Bar {
    /// The keymap focuses this box by name, which is the only thing outside here that
    /// needs to know it exists.
    pub const SEARCH: &'static str = "q";

    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        let theme = session.engine.ui().theme();
        egui::TopBottomPanel::top("bar")
            .frame(floating(theme))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(session.engine.ui().count()).color(color(theme.muted)));
                    ui.separator();
                    let box_ = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .id(Id::new(Bar::SEARCH))
                            .hint_text("Search…  /")
                            .desired_width(220.0),
                    );
                    if box_.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        session.engine.search(&self.query);
                    }
                    ui.separator();
                    let rescan = ui
                        .button(glyph::RESCAN)
                        .on_hover_text("Rescan the vault for new and changed notes");
                    if rescan.clicked() {
                        session.rescan(ctx);
                    }
                    if ui
                        .button(glyph::SETTINGS)
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        session.engine.ui_mut().open_settings();
                    }
                });
            });
    }
}
