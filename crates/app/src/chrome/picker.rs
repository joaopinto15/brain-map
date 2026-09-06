//! The start page. Nothing is open until this names something, so it is the one panel a
//! first run always sees: what can be opened is on it, not in the README.
//!
//! One field takes all three — a folder, a git URL, a drive — because the scanner decides
//! which it is, and a bare word searches the vaults already opened instead. The folder
//! dialog is the desktop's, since typing a path is not how anyone finds one.

use super::floating;
use crate::app::Session;
use crate::filter::{is_search, vault_search};
use crate::theme::color;
use eframe::egui::{self, Align2, Context, Key, RichText};

/// What a source looks like, shown under the field so the syntax is never guessed at.
const EXAMPLES: [(&str, &str); 3] = [
    ("Folder", "~/notes"),
    ("Git", "https://github.com/you/notes"),
    ("Drive", "gdrive:notes"),
];

#[derive(Default)]
pub struct Picker {
    /// What is being typed — a location to open, or a word to search for — and what went
    /// wrong with the last one. Both belong to the picker and to nothing else.
    typed: String,
    error: String,
    /// The configured drives, asked for once: running rclone every frame would be silly.
    drives: Option<Vec<String>>,
    /// Set when something else filled the field, so the caret goes back to it.
    refocus: bool,
}

impl Picker {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        if !session.engine.ui().picker_open() {
            return;
        }
        let drives = self.drives.get_or_insert_with(|| session.drives()).clone();
        let theme = session.engine.ui().theme();
        let recent = session.engine.ui().recent().to_vec();
        let found = vault_search(&recent, &self.typed);
        // A word takes the vault it found; anything else is a location, and is opened as
        // it was typed.
        let entered = match is_search(&self.typed) {
            true => found.first().map(|hit| hit.to_string()),
            false => Some(self.typed.clone()),
        };
        let mut open = None;
        let mut browse = false;

        egui::Window::new("brain-map")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(floating(theme).inner_margin(egui::Margin::same(20)))
            .show(ctx, |ui| {
                ui.set_width(430.0);
                ui.heading(RichText::new("brain-map").color(color(theme.heading)));
                ui.label(
                    RichText::new("Open a vault of markdown notes, or search the ones you have.")
                        .color(color(theme.muted)),
                );

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut self.typed)
                            .hint_text("Search, or a path · URL · drive")
                            .desired_width(250.0),
                    );
                    // Focused when nothing else is, rather than every frame: the field
                    // is what you type into, but a button that was clicked keeps its say.
                    if self.refocus || ui.memory(|m| m.focused().is_none()) {
                        field.request_focus();
                        self.refocus = false;
                    }
                    if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        open = entered.clone();
                    }
                    browse = ui.button("Browse…").clicked();
                    if ui.button("Open").clicked() {
                        open = entered.clone();
                    }
                });

                ui.add_space(10.0);
                egui::Grid::new("examples")
                    .num_columns(2)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        for (kind, example) in EXAMPLES {
                            ui.label(RichText::new(kind).size(11.0).color(color(theme.accent)));
                            ui.label(
                                RichText::new(example)
                                    .size(11.0)
                                    .monospace()
                                    .color(color(theme.muted)),
                            );
                            ui.end_row();
                        }
                    });

                if !drives.is_empty() {
                    ui.add_space(12.0);
                    section(ui, theme, "Drives");
                    ui.horizontal_wrapped(|ui| {
                        for drive in &drives {
                            // Filling the field rather than opening it: the whole drive is
                            // rarely the vault, and a folder on it is one word away.
                            let chip = ui.button(RichText::new(drive).monospace());
                            if chip
                                .on_hover_text("Fill the field, then name a folder on it")
                                .clicked()
                            {
                                self.typed = drive.clone();
                                self.refocus = true;
                            }
                        }
                    });
                }

                if !recent.is_empty() {
                    ui.add_space(12.0);
                    section(
                        ui,
                        theme,
                        &match found.len() == recent.len() {
                            true => "Recent".to_string(),
                            false => format!("Recent · {} of {}", found.len(), recent.len()),
                        },
                    );
                    for path in &found {
                        let row = ui.add(
                            egui::Button::selectable(false, shorten(path))
                                .truncate()
                                .min_size(egui::vec2(ui.available_width(), 0.0)),
                        );
                        if row.on_hover_text(*path).clicked() {
                            open = Some(path.to_string());
                        }
                    }
                    if found.is_empty() {
                        ui.label(
                            RichText::new("no vault of yours matches — Enter opens what you typed")
                                .size(10.5)
                                .color(color(theme.muted)),
                        );
                    }
                }

                if !self.error.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new(&self.error).color(color(theme.code)));
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
        if let Some(typed) = open.filter(|t| !t.trim().is_empty()) {
            match session.open_vault(ctx, &typed) {
                Ok(()) => {
                    self.error.clear();
                    self.typed.clear();
                }
                Err(said) => self.error = said,
            }
        }
    }
}

fn section(ui: &mut egui::Ui, theme: &crate::theme::Theme, title: &str) {
    ui.label(
        RichText::new(title)
            .size(10.5)
            .strong()
            .color(color(theme.muted)),
    );
    ui.add_space(2.0);
}

/// `$HOME` back to `~`, so a row reads as the path you would have typed.
fn shorten(path: &str) -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => path.replace(&home, "~"),
        _ => path.to_string(),
    }
}
