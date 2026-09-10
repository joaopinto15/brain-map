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
pub struct Explorer {
    /// Where the open note is scrolled to. The panel owns it because only the panel needs
    /// it, and a key that scrolls the reader has to know where it is scrolling from.
    note_scroll: f32,
}

/// What the panel was asked to do, gathered while it draws and acted on after — the panel
/// borrows the session to render, so it cannot also change it mid-frame.
#[derive(Default)]
struct Asked {
    go_to: Option<usize>,
    /// The folder a click asked to open or shut, and the row it put the cursor on.
    folded: Option<String>,
    pointed: Option<tree::Key>,
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
        // Taken before the tree draws: the row the keyboard moved to scrolls itself into
        // view on that one frame and leaves the scrollbar alone afterwards.
        let chase = session.engine.ui_mut().take_chase();
        egui::SidePanel::left("explorer")
            .frame(floating(theme).inner_margin(egui::Margin::same(10)))
            .resizable(false)
            .exact_width(width)
            .show(ctx, |ui| {
                Reader::new(theme).style(ui);
                match session.engine.ui().reading() {
                    None => {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            tree_of(
                                ui,
                                &session.engine,
                                &mut session.icons,
                                session.engine.tree(),
                                "",
                                chase,
                                &mut asked,
                            );
                        });
                    }
                    Some(at) => note(ui, session, at, &mut asked, &mut self.note_scroll),
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
        if let Some(key) = asked.pointed {
            session.engine.ui_mut().point_at(key);
        }
        if let Some(path) = asked.folded {
            session.engine.ui_mut().toggle_dir(&path);
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
        // Written when the grip is let go, not while it moves: a drag changes the width
        // every frame and the settings file is rewritten whole.
        if dragged.drag_stopped() {
            session.engine.remember_panel();
        }
    }
}

fn note(ui: &mut egui::Ui, session: &mut Session, at: usize, asked: &mut Asked, scroll: &mut f32) {
    let theme = session.engine.ui().theme();
    let node = session.engine.node(at);
    let (label, id) = (node.label.clone(), node.id.clone());
    let (concept, signals) = (node.group.clone(), node.signals.clone());
    // The buttons take their width first and the title elides into what is left: a
    // heading laid out before them wraps to the whole panel and pushes them off its edge.
    ui.horizontal(|ui| {
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
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&label).heading().color(color(theme.heading)))
                        .truncate(),
                )
                .on_hover_text(&label);
            });
        });
    });
    ui.label(RichText::new(&id).size(10.5).color(color(theme.muted)));
    // What the concept declared about itself: its type, then the trust tier and the
    // lifecycle the legend filters on. Nothing here is derived from the body.
    ui.horizontal_wrapped(|ui| {
        let graph = session.engine.graph();
        if let Some(group) = graph.group(&concept) {
            ui.label(
                RichText::new(&group.name)
                    .size(11.0)
                    .color(theme.group_colour(&graph.groups, &concept)),
            );
        }
        for signal in &signals {
            ui.add_space(8.0);
            ui.label(RichText::new(signal).size(11.0).color(color(theme.muted)));
        }
    });
    let tags = session.engine.node(at).tags.clone();
    if !tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for tag in &tags {
                ui.label(RichText::new(tag).size(11.0).color(color(theme.accent)));
                ui.add_space(8.0);
            }
        });
    }
    let about = session.engine.concept(at).clone();
    if !about.description.is_empty() {
        ui.add_space(4.0);
        ui.label(RichText::new(&about.description).italics());
    }
    if !about.resource.is_empty() && ui.link(RichText::new(&about.resource).size(11.0)).clicked() {
        asked.followed = Some(Target::Url(about.resource.clone()));
    }
    ui.separator();
    // What `j` and `k` asked for while the note was open. The offset is set rather than
    // nudged: `Ui::scroll_with_delta` leaves it in the pass state, where the first scroll
    // area to finish takes it — and inside a note that is a table or a code block, which
    // only scroll sideways.
    let scrolled = session.engine.ui_mut().take_scroll();
    let mut area = egui::ScrollArea::vertical().id_salt("note");
    if scrolled != 0.0 {
        area = area.vertical_scroll_offset((*scroll - scrolled).max(0.0));
    }
    let shown = area.show(ui, |ui| {
        match session.note() {
            Some(blocks) if !blocks.is_empty() => {
                asked.followed = Reader::new(theme).show(ui, blocks);
            }
            Some(_) => {
                ui.label(RichText::new("This note is empty.").color(color(theme.muted)));
            }
            None => {
                ui.label(RichText::new("Loading…").color(color(theme.muted)));
            }
        }
        provenance(ui, session, at, &id, &about, asked);
    });
    *scroll = shown.state.offset.y;
}

/// Where the concept came from and who stands behind it, under the body the way the OKF
/// viewer lists them: who generated and verified it, the sources it derives from, and the
/// concepts that cite it. A source that names a note in this vault opens it; a URL opens
/// outside; a scope descriptor is just words.
fn provenance(
    ui: &mut egui::Ui,
    session: &Session,
    at: usize,
    id: &str,
    about: &brain_map_model::Concept,
    asked: &mut Asked,
) {
    let theme = session.engine.ui().theme();
    let muted = |text: &str| RichText::new(text).size(11.0).color(color(theme.muted));
    let heading = |ui: &mut egui::Ui, text: &str| {
        ui.add_space(8.0);
        ui.label(
            RichText::new(text)
                .size(11.0)
                .strong()
                .color(color(theme.muted)),
        );
    };
    let actor = |event: &brain_map_model::Actor| match (event.by.is_empty(), event.at.is_empty()) {
        (false, false) => format!("{} · {}", event.by, event.at),
        (false, true) => event.by.clone(),
        (true, _) => event.at.clone(),
    };

    if about.generated.is_some() || !about.verified.is_empty() {
        heading(ui, "Trust");
        if let Some(generated) = &about.generated {
            ui.label(muted(&format!("generated {}", actor(generated))));
        }
        for event in &about.verified {
            ui.label(muted(&format!("verified {}", actor(event))));
        }
    }

    if !about.sources.is_empty() {
        heading(ui, "Sources");
        for source in &about.sources {
            let label = [&source.title, &source.resource, &source.id]
                .into_iter()
                .find(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| "source".to_string());
            let resource = source.resource.as_str();
            let note = session
                .engine
                .resolve(resource, Some(id), true)
                .or_else(|| session.engine.resolve(resource, None, false));
            let is_url = resource.starts_with("http://") || resource.starts_with("https://");
            match (note, is_url) {
                (Some(node), _) => {
                    if ui.link(RichText::new(&label).size(11.0)).clicked() {
                        asked.go_to = Some(node);
                    }
                }
                (None, true) => {
                    if ui.link(RichText::new(&label).size(11.0)).clicked() {
                        asked.followed = Some(Target::Url(resource.to_string()));
                    }
                }
                (None, false) => {
                    ui.label(muted(&label));
                }
            }
        }
    }

    let cited_by = session.engine.cited_by(at);
    if !cited_by.is_empty() {
        heading(ui, "Cited by");
        for from in cited_by {
            let name = session.engine.node(from).label.clone();
            if ui.link(RichText::new(name).size(11.0)).clicked() {
                asked.go_to = Some(from);
            }
        }
    }
}

/// The vault as a folder tree. Which folders are open lives in `Ui` rather than in the
/// header's own memory, because the keyboard opens and shuts them too — `Engine::rows`
/// flattens the same answer into the list `j` and `k` walk.
fn tree_of(
    ui: &mut egui::Ui,
    engine: &Engine,
    icons: &mut Icons,
    dir: &tree::Dir,
    path: &str,
    chase: bool,
    asked: &mut Asked,
) {
    let cursor = engine.ui().cursor();
    let theme = engine.ui().theme();
    for (name, below) in &dir.dirs {
        let here = format!("{path}/{name}");
        let on = cursor == Some(&tree::Key::Dir(here.clone()));
        // A collapsing header lays its title out with `TextWrapMode::Extend` and offers
        // no way to say otherwise, so the name is cut to fit before it is handed over.
        let room = ui.available_width() - ui.spacing().indent - HEADER_PAD;
        let mut title = RichText::new(elide(name, room, |text| width_of(ui, text)));
        if on {
            title = title.color(color(theme.accent));
        }
        // The header's own arrow says it is a folder, so it needs no icon of its own.
        let open = egui::CollapsingHeader::new(title)
            .id_salt(&here)
            .open(Some(engine.ui().dir_open(&here)))
            .show(ui, |ui| {
                tree_of(ui, engine, icons, below, &here, chase, asked)
            });
        let header = open.header_response.on_hover_text(name);
        if on && chase {
            header.scroll_to_me(Some(egui::Align::Center));
        }
        if header.clicked() {
            asked.folded = Some(here.clone());
            asked.pointed = Some(tree::Key::Dir(here));
        }
    }
    for &node in &dir.files {
        let on = engine.ui().reading() == Some(node) || cursor == Some(&tree::Key::File(node));
        ui.horizontal(|ui| {
            icon(ui, icons, engine.icon_of(node), 13.0);
            // A name too long for the panel ends in an ellipsis rather than being cut off
            // mid-letter, and the whole of it is the tooltip.
            let row = ui.add(egui::Button::selectable(on, &engine.node(node).label).truncate());
            let row = row.on_hover_text(engine.node_id(node));
            if on && chase {
                row.scroll_to_me(Some(egui::Align::Center));
            }
            if row.clicked() {
                asked.pointed = Some(tree::Key::File(node));
                asked.go_to = Some(node);
            }
        });
    }
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
