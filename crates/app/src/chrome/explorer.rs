//! The explorer, and the note when one is open. Two states, as the reader has always had:
//! reading swaps the folder tree for the note, full gives the panel the window.
//!
//! The panel's width is dragged from its own edge and typed in the settings dialog, and
//! both go through `Engine::set_panel` — the width is one number with one writer.

use super::{floating, glyph, icon};
use crate::app::Session;
use crate::emoji::Icons;
use crate::markdown::Target;
use crate::reader::Reader;
use crate::state::Engine;
use crate::theme::color;
use crate::tree;
use eframe::egui::{self, Context, Id, RichText, Sense};

/// How wide the drag strip down the panel's edge is.
const GRIP: f32 = 6.0;
/// What a collapsing header spends on its own padding, which a title cannot have.
const HEADER_PAD: f32 = 8.0;

#[derive(Default)]
pub struct Explorer;

/// What the panel was asked to do, gathered while it draws and acted on after — the panel
/// borrows the session to render, so it cannot also change it mid-frame.
#[derive(Default)]
struct Asked {
    go_to: Option<usize>,
    followed: Option<Target>,
    close: bool,
    edit: bool,
    full: bool,
}

impl Explorer {
    pub fn show(&mut self, ctx: &Context, session: &mut Session) {
        if session.engine.ui().explorer_hidden() {
            return;
        }
        let theme = session.engine.ui().theme();
        let width = match session.engine.ui().is_full() {
            true => ctx.screen_rect().width(),
            false => session.engine.ui().panel_width() as f32,
        };
        let mut asked = Asked::default();
        egui::SidePanel::left("explorer")
            .frame(floating(theme).inner_margin(egui::Margin::same(10)))
            .resizable(false)
            .exact_width(width)
            .show(ctx, |ui| {
                Reader::new(theme).style(ui);
                match session.engine.ui().reading() {
                    None => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            asked.go_to = tree_of(
                                ui,
                                &session.engine,
                                &mut session.icons,
                                session.engine.tree(),
                                "",
                            );
                        });
                    }
                    Some(at) => note(ui, session, at, &mut asked),
                }
            });
        self.grip(ctx, session);

        if asked.full {
            session.engine.ui_mut().toggle_full();
        }
        if asked.close {
            session.engine.close_note();
        }
        if asked.edit {
            session.edit_open_note();
        }
        if let Some(node) = asked.go_to {
            session.engine.go_to(node);
        }
        if let Some(target) = asked.followed {
            session.follow(target);
        }
    }

    fn grip(&mut self, ctx: &Context, session: &mut Session) {
        if session.engine.ui().is_full() {
            return;
        }
        let screen = ctx.screen_rect();
        let x = session.engine.ui().panel_width() as f32;
        let dragged = egui::Area::new(Id::new("grip"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(x - GRIP / 2.0, screen.top()))
            .show(ctx, |ui| {
                let (_, response) =
                    ui.allocate_exact_size(egui::vec2(GRIP, screen.height()), Sense::drag());
                response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            })
            .inner;
        if dragged.dragged() {
            if let Some(pointer) = ctx.pointer_latest_pos() {
                session.engine.set_panel(pointer.x as f64);
            }
        }
        if dragged.drag_stopped() {
            session.engine.ui_mut().save_panel_width();
        }
    }
}

fn note(ui: &mut egui::Ui, session: &mut Session, at: usize, asked: &mut Asked) {
    let theme = session.engine.ui().theme();
    let node = session.engine.node(at);
    let (label, id) = (node.label.clone(), node.id.clone());
    ui.horizontal(|ui| {
        ui.heading(RichText::new(&label).color(color(theme.heading)));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            asked.close = ui
                .button(glyph::CLOSE)
                .on_hover_text("Close (Esc)")
                .clicked();
            asked.edit = ui
                .button(glyph::EDIT)
                .on_hover_text("Open in $EDITOR")
                .clicked();
            asked.full = ui
                .button(glyph::FULLSCREEN)
                .on_hover_text("Fullscreen (F)")
                .clicked();
        });
    });
    ui.label(RichText::new(&id).size(10.5).color(color(theme.muted)));
    let tags = session.engine.node(at).tags.clone();
    if !tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for tag in &tags {
                ui.label(RichText::new(tag).size(11.0).color(color(theme.accent)));
                ui.add_space(8.0);
            }
        });
    }
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt("note")
        .show(ui, |ui| match session.note() {
            Some(blocks) if !blocks.is_empty() => {
                asked.followed = Reader::new(theme).show(ui, blocks);
            }
            Some(_) => {
                ui.label(RichText::new("This note is empty.").color(color(theme.muted)));
            }
            None => {
                ui.label(RichText::new("Loading…").color(color(theme.muted)));
            }
        });
}

/// The vault as a folder tree. A collapsing header carries the open and closed state, so
/// there is none to track here.
fn tree_of(
    ui: &mut egui::Ui,
    engine: &Engine,
    icons: &mut Icons,
    dir: &tree::Dir,
    path: &str,
) -> Option<usize> {
    let mut chosen = None;
    for (name, below) in &dir.dirs {
        let here = format!("{path}/{name}");
        // A collapsing header lays its title out with `TextWrapMode::Extend` and offers
        // no way to say otherwise, so the name is cut to fit before it is handed over.
        let room = ui.available_width() - ui.spacing().indent - HEADER_PAD;
        let title = elide(name, room, |text| width_of(ui, text));
        // The header's own arrow says it is a folder, so it needs no icon of its own.
        let open = egui::CollapsingHeader::new(title)
            .id_salt(&here)
            .default_open(true)
            .show(ui, |ui| tree_of(ui, engine, icons, below, &here));
        open.header_response.on_hover_text(name);
        chosen = open.body_returned.flatten().or(chosen);
    }
    for &node in &dir.files {
        let on = engine.ui().reading() == Some(node);
        ui.horizontal(|ui| {
            icon(ui, icons, engine.icon_of(node), 13.0);
            // A name too long for the panel ends in an ellipsis rather than being cut off
            // mid-letter, and the whole of it is the tooltip.
            let row = ui.add(egui::Button::selectable(on, &engine.node(node).label).truncate());
            if row.on_hover_text(engine.node_id(node)).clicked() {
                chosen = Some(node);
            }
        });
    }
    chosen
}

/// How wide a title is drawn, asked of the fonts that will draw it.
fn width_of(ui: &egui::Ui, text: &str) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    ui.fonts(|fonts| {
        fonts
            .layout_no_wrap(text.to_string(), font, egui::Color32::WHITE)
            .size()
            .x
    })
}

/// A name cut to fit, ending in an ellipsis. `measure` is supplied so the rule can be
/// checked without a window, the way the simulation measures its labels.
fn elide(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> String {
    if width <= 0.0 || measure(text) <= width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    // The most characters that still fit alongside the ellipsis. Bisected rather than
    // walked, so a long name costs a handful of measurements and not one per letter.
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate: String = chars[..mid].iter().chain(['…'].iter()).collect();
        match measure(&candidate) <= width {
            true => low = mid,
            false => high = mid - 1,
        }
    }
    chars[..low].iter().chain(['…'].iter()).collect()
}

#[cfg(test)]
mod tests {
    use super::elide;

    /// Ten pixels a character, so the arithmetic in the test is the arithmetic a reader
    /// can do in their head.
    fn ten(text: &str) -> f32 {
        text.chars().count() as f32 * 10.0
    }

    #[test]
    fn a_name_too_long_for_the_panel_ends_in_an_ellipsis() {
        assert_eq!(
            elide("note", 100.0, ten),
            "note",
            "one that fits is left alone"
        );
        assert_eq!(
            elide("note", 40.0, ten),
            "note",
            "and so is one that exactly fits"
        );
        // Eleven characters at ten pixels, with room for five: four of them and the dot.
        assert_eq!(elide("a long name", 50.0, ten), "a lo…");
        assert_eq!(
            elide("a long name", 10.0, ten),
            "…",
            "room for the ellipsis alone"
        );
        assert_eq!(
            elide("a long name", 0.0, ten),
            "a long name",
            "no room is not a width"
        );
        // Cutting happens by character, so a name is never split through one.
        assert_eq!(elide("héllo wörld", 50.0, ten), "héll…");
    }
}
