//! Every setting, in one dialog. A setting reachable two ways reads its state back out of
//! [`crate::state::Ui`] rather than keeping a copy: the width is also the grip, and the
//! explorer switch is also `space e`.

use super::floating;
use crate::app::Session;
use crate::sim::GROWTH_MS;
use crate::state::MAX_PANEL;
use crate::theme::{color, THEMES};
use eframe::egui::{self, Align2, Context, Key, RichText};

/// How long the whole vault may take to assemble.
const BUDGETS: [(u32, &str); 6] = [
    (0, "Instant"),
    (3000, "3s"),
    (5000, "5s"),
    (GROWTH_MS, "8s"),
    (15000, "15s"),
    (30000, "30s"),
];

#[derive(Default)]
pub struct SettingsDialog;

/// What the dialog was set to this frame. Applied after it closes, so a widget is never
/// read back in the same frame it was written.
struct Chosen {
    theme: usize,
    budget: u32,
    width: f64,
    hidden: bool,
    pick_vault: bool,
}

impl SettingsDialog {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        if !session.engine.ui().settings_open() {
            return;
        }
        let theme = session.engine.ui().theme();
        let ui_state = session.engine.ui();
        let mut chosen = Chosen {
            theme: ui_state.theme_at(),
            budget: ui_state.budget(),
            width: ui_state.panel_width(),
            hidden: ui_state.explorer_hidden(),
            pick_vault: false,
        };
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(floating(theme).inner_margin(egui::Margin::same(14)))
            .show(ctx, |ui| {
                egui::Grid::new("settings")
                    .num_columns(2)
                    .spacing([12.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Vault");
                        ui.horizontal(|ui| {
                            let vault = &session.engine.graph().vault;
                            ui.label(
                                RichText::new(match vault.is_empty() {
                                    true => "none chosen",
                                    false => vault.as_str(),
                                })
                                .color(color(theme.muted)),
                            );
                            chosen.pick_vault = ui.button("Change…").clicked();
                        });
                        ui.end_row();

                        ui.label("Theme");
                        egui::ComboBox::from_id_salt("theme")
                            .selected_text(THEMES[chosen.theme].name)
                            .show_ui(ui, |ui| {
                                for (at, entry) in THEMES.iter().enumerate() {
                                    ui.selectable_value(&mut chosen.theme, at, entry.name);
                                }
                            });
                        ui.end_row();

                        ui.label("Growth animation");
                        egui::ComboBox::from_id_salt("speed")
                            .selected_text(
                                BUDGETS
                                    .iter()
                                    .find(|(ms, _)| *ms == chosen.budget)
                                    .map_or("custom", |(_, name)| name),
                            )
                            .show_ui(ui, |ui| {
                                for (ms, name) in BUDGETS {
                                    ui.selectable_value(&mut chosen.budget, ms, name);
                                }
                            });
                        ui.end_row();

                        ui.label("Explorer width");
                        ui.add(
                            egui::Slider::new(&mut chosen.width, 220.0..=MAX_PANEL)
                                .suffix("px")
                                .step_by(10.0),
                        );
                        ui.end_row();

                        ui.label("Show explorer");
                        ui.horizontal(|ui| {
                            let mut showing = !chosen.hidden;
                            ui.checkbox(&mut showing, "");
                            chosen.hidden = !showing;
                            ui.label(
                                RichText::new("space e")
                                    .size(10.5)
                                    .color(color(theme.muted)),
                            );
                        });
                        ui.end_row();
                    });
            });

        self.apply(ctx, session, chosen);
        // The window's own close button, or Esc, which is what a dialog has always
        // closed on.
        if !open || ctx.input(|i| i.key_pressed(Key::Escape)) {
            session.engine.ui_mut().close_settings();
        }
    }

    fn apply(&self, ctx: &Context, session: &mut Session, chosen: Chosen) {
        if chosen.theme != session.engine.ui().theme_at() {
            session.engine.ui_mut().set_theme(chosen.theme);
            ctx.set_visuals(session.engine.ui().theme().visuals());
        }
        if chosen.budget != session.engine.ui().budget() {
            session.engine.set_budget(chosen.budget);
        }
        if chosen.width != session.engine.ui().panel_width() {
            session.engine.set_panel(chosen.width);
            session.engine.ui_mut().save_panel_width();
        }
        if chosen.hidden != session.engine.ui().explorer_hidden() {
            session.engine.hide_panel(chosen.hidden);
        }
        if chosen.pick_vault {
            let ui = session.engine.ui_mut();
            ui.close_settings();
            ui.open_picker();
        }
    }
}
