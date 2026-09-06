//! Choosing the vault. Nothing is open until this names something, and the folder dialog
//! is the desktop's because typing a path is not how anyone finds one. A git URL and an
//! rclone remote name one too: the scanner fetches either into the cache and opens the
//! copy, so importing a vault is the same field and the same button as opening one.

use super::floating;
use crate::app::Session;
use crate::theme::color;
use eframe::egui::{self, Align2, Context, Key, RichText};

#[derive(Default)]
pub struct Picker {
    /// The path being typed, and what went wrong with the last one — both belong to the
    /// picker and to nothing else.
    typed: String,
    error: String,
}

impl Picker {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        if !session.engine.ui().picker_open() {
            return;
        }
        let theme = session.engine.ui().theme();
        let mut open = None;
        let mut browse = false;
        egui::Window::new("brain-map")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(floating(theme).inner_margin(egui::Margin::same(18)))
            .show(ctx, |ui| {
                ui.heading(RichText::new("brain-map").color(color(theme.heading)));
                ui.label(
                    RichText::new(
                        "Open a folder of markdown notes as a vault — or a git URL or a drive to import one.",
                    )
                    .color(color(theme.muted)),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.typed)
                            .hint_text("~/notes · https://github.com/you/notes · gdrive:notes")
                            .desired_width(340.0),
                    );
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        open = Some(self.typed.clone());
                    }
                    browse = ui.button("Browse…").clicked();
                    if ui.button("Open").clicked() {
                        open = Some(self.typed.clone());
                    }
                });
                if !self.error.is_empty() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(&self.error).color(color(theme.code)));
                }
                if !session.engine.ui().recent().is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("Recent").strong());
                    for path in session.engine.ui().recent().to_vec() {
                        if ui.selectable_label(false, &path).clicked() {
                            open = Some(path);
                        }
                    }
                }
            });

        if browse {
            self.error.clear();
            match session.choose_folder() {
                Ok(Some(chosen)) => {
                    self.typed = chosen.clone();
                    open = Some(chosen);
                }
                Ok(None) => self.error = "no folder chosen".into(),
                Err(said) => self.error = said,
            }
        }
        if let Some(typed) = open {
            match session.open_vault(ctx, &typed) {
                Ok(()) => self.error.clear(),
                Err(said) => self.error = said,
            }
        }
    }
}
