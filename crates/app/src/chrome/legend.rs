//! The legend: what the colours mean, and the one place a filter is turned on.
//!
//! A type row, a signal row and a tag row all do the same thing to the graph — they are
//! the three axes an OKF bundle is cut along, so they are one list with a rule between
//! them: what a concept *is*, how far it is trusted, and what it is about.

use super::floating;
use crate::app::Session;
use crate::filter::Filter;
use crate::theme::color;
use eframe::egui::{self, Align2, Color32, Context, RichText, Sense};

#[derive(Default)]
pub struct Legend;

impl Legend {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        let theme = session.engine.ui().theme();
        // Collected before the window, so a row can be clicked without the filter being
        // written while the list it came from is still being read.
        let mut picked = None;
        egui::Window::new("legend")
            .title_bar(false)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, [-16.0, 16.0])
            // A narrow window gets a narrow legend: the rows elide rather than eat the
            // graph, and the widest tag no longer decides how much of it is covered.
            .max_width((ctx.screen_rect().width() * 0.2).clamp(120.0, 260.0))
            .frame(floating(theme).inner_margin(egui::Margin::same(10)))
            .show(ctx, |ui| {
                // The window is anchored to the top, so what is left below it is all
                // the room a short window has for the rows.
                let rows = ctx.screen_rect().height() - 2.0 * 16.0 - 44.0;
                egui::ScrollArea::vertical()
                    .max_height(rows.max(60.0))
                    .show(ui, |ui| {
                        let graph = session.engine.graph();
                        for (group, count) in graph.groups.iter().zip(session.engine.group_counts())
                        {
                            if *count == 0 {
                                continue;
                            }
                            let on = session.engine.ui().filter()
                                == Some(&Filter::Group(group.key.clone()));
                            ui.horizontal(|ui| {
                                dot(ui, theme.group_colour(&graph.groups, &group.key));
                                if ui
                                    .selectable_label(on, format!("{} · {count}", group.name))
                                    .clicked()
                                {
                                    picked = Some(Filter::Group(group.key.clone()));
                                }
                            });
                        }
                        if !session.engine.signal_counts().is_empty() {
                            ui.separator();
                            // Trust and lifecycle: a concept's own frontmatter said so, which is
                            // why a bundle that declares nothing shows nothing here.
                            for (signal, count) in session.engine.signal_counts() {
                                let on = session.engine.ui().filter()
                                    == Some(&Filter::Signal(signal.clone()));
                                if ui
                                    .selectable_label(on, format!("{signal} · {count}"))
                                    .clicked()
                                {
                                    picked = Some(Filter::Signal(signal.clone()));
                                }
                            }
                        }
                        if !session.engine.top_tags().is_empty() {
                            ui.separator();
                            for (tag, count) in session.engine.top_tags() {
                                // A tag has no icon of its own: an icon belongs to a note, which
                                // is the only place one is ever written.
                                let on =
                                    session.engine.ui().filter() == Some(&Filter::Tag(tag.clone()));
                                if ui
                                    .selectable_label(on, format!("{tag} · {count}"))
                                    .clicked()
                                {
                                    picked = Some(Filter::Tag(tag.clone()));
                                }
                            }
                        }
                    });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(match session.engine.ui().filter().is_some() {
                        true => "click again or Esc to clear",
                        false => "click a row to filter",
                    })
                    .size(10.5)
                    .color(color(theme.muted)),
                );
            });
        if let Some(chosen) = picked {
            session.engine.ui_mut().toggle_filter(chosen);
        }
    }
}

fn dot(ui: &mut egui::Ui, colour: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, colour);
}
