//! The legend: what the colours mean, and the one place a filter is turned on.
//!
//! A group row and a tag row do the same thing to the graph — they are the two axes a
//! vault is cut along, so they are one list with a rule between them.

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
            .frame(floating(theme).inner_margin(egui::Margin::same(10)))
            .show(ctx, |ui| {
                let graph = session.engine.graph();
                for (group, count) in graph.groups.iter().zip(session.engine.group_counts()) {
                    if *count == 0 {
                        continue;
                    }
                    let on =
                        session.engine.ui().filter() == Some(&Filter::Group(group.key.clone()));
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
                if !session.engine.top_tags().is_empty() {
                    ui.separator();
                    for (tag, count) in session.engine.top_tags() {
                        // A tag has no icon of its own: an icon belongs to a note, which
                        // is the only place one is ever written.
                        let on = session.engine.ui().filter() == Some(&Filter::Tag(tag.clone()));
                        if ui
                            .selectable_label(on, format!("{tag} · {count}"))
                            .clicked()
                        {
                            picked = Some(Filter::Tag(tag.clone()));
                        }
                    }
                }
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
