//! The note, drawn. [`crate::markdown`] decided what the blocks are; this decides what
//! they look like, and hands back the link if one was clicked.
//!
//! Inline runs are laid out as separate widgets inside a wrapping row, which is what
//! makes a link inside a sentence clickable without the sentence stopping being a
//! sentence.

use crate::markdown::{Align, Block, Span, Target};
use crate::theme::{color, Theme};
use eframe::egui::{self, FontId, RichText, TextStyle, Ui};

/// Heading sizes, largest first. A note that goes deeper than six gets the smallest.
const HEADINGS: [f32; 6] = [22.0, 18.0, 15.5, 14.0, 13.0, 12.0];
const INDENT: f32 = 18.0;

pub struct Reader {
    theme: &'static Theme,
}

impl Reader {
    pub fn new(theme: &'static Theme) -> Reader {
        Reader { theme }
    }

    /// The reader's own text sizes, so a note reads as prose rather than as chrome.
    pub fn style(&self, ui: &mut Ui) {
        let styles = &mut ui.style_mut().text_styles;
        styles.insert(TextStyle::Body, FontId::proportional(13.5));
        styles.insert(TextStyle::Monospace, FontId::monospace(12.0));
    }

    pub fn show(&self, ui: &mut Ui, blocks: &[Block]) -> Option<Target> {
        let theme = self.theme;
        let mut followed = None;
        for (at, block) in blocks.iter().enumerate() {
            match block {
                Block::Heading(level, spans) => {
                    ui.add_space(6.0);
                    let size = HEADINGS[(*level - 1).min(HEADINGS.len() - 1)];
                    followed = self
                        .runs(
                            ui,
                            spans,
                            Some(FontId::proportional(size)),
                            Some(color(theme.heading)),
                        )
                        .or(followed);
                    ui.add_space(2.0);
                }
                Block::Para(spans) => {
                    followed = self.runs(ui, spans, None, None).or(followed);
                    ui.add_space(6.0);
                }
                Block::Quote(spans) => {
                    // The bar down the side is the quote: egui has no border on a label, and
                    // an indent alone reads as a list.
                    let struck = ui.horizontal(|ui| {
                        let top = ui.cursor().top();
                        ui.add_space(4.0);
                        let struck = ui
                            .vertical(|ui| self.runs(ui, spans, None, Some(color(theme.muted))))
                            .inner;
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(ui.min_rect().left(), top),
                            egui::pos2(ui.min_rect().left() + 2.0, ui.min_rect().bottom()),
                        );
                        ui.painter().rect_filled(bar, 1.0, color(theme.accent));
                        struck
                    });
                    followed = struck.inner.or(followed);
                    ui.add_space(6.0);
                }
                Block::Code { text, .. } => {
                    egui::Frame::new()
                        .fill(color(theme.input_bg))
                        .inner_margin(8.0)
                        .corner_radius(4.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::horizontal()
                                .id_salt(("code", at))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(text)
                                                .monospace()
                                                .color(color(theme.code)),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                });
                        });
                    ui.add_space(6.0);
                }
                Block::Item {
                    indent,
                    marker,
                    body,
                } => {
                    let struck = ui.horizontal_top(|ui| {
                        ui.add_space(*indent as f32 * INDENT);
                        ui.label(RichText::new(marker).color(color(theme.muted)));
                        ui.vertical(|ui| self.runs(ui, body, None, None)).inner
                    });
                    followed = struck.inner.or(followed);
                    ui.add_space(2.0);
                }
                Block::Rule => {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                }
                Block::Table { align, head, rows } => {
                    // A link inside a cell is a link like any other: it was being drawn
                    // and then dropped on the floor.
                    let mut struck = None;
                    egui::ScrollArea::horizontal()
                        .id_salt(("table", at))
                        .show(ui, |ui| {
                            egui::Grid::new(("grid", at)).striped(true).show(ui, |ui| {
                                for (column, cell) in head.iter().enumerate() {
                                    let at = align.get(column).copied().unwrap_or_default();
                                    if let Some(t) = self.cell(ui, cell, at, true) {
                                        struck = Some(t);
                                    }
                                }
                                ui.end_row();
                                for row in rows {
                                    for (column, cell) in row.iter().enumerate() {
                                        let at = align.get(column).copied().unwrap_or_default();
                                        if let Some(t) = self.cell(ui, cell, at, false) {
                                            struck = Some(t);
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                    followed = struck.or(followed);
                    ui.add_space(6.0);
                }
            }
        }
        followed
    }

    /// One table cell. `line` puts the runs in a single horizontal group, so the
    /// alignment layout places that one group and never reorders the words inside it.
    fn cell(&self, ui: &mut Ui, cell: &[Span], align: Align, head: bool) -> Option<Target> {
        let layout = match align {
            Align::Start => egui::Layout::left_to_right(egui::Align::Center),
            Align::Center => egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            Align::End => egui::Layout::right_to_left(egui::Align::Center),
        };
        let colour = head.then(|| color(self.theme.heading));
        ui.with_layout(layout, |ui| self.line(ui, cell, colour))
            .inner
    }

    /// Prose: the runs wrap at the panel's width, the way a paragraph should.
    fn runs(
        &self,
        ui: &mut Ui,
        spans: &[Span],
        font: Option<FontId>,
        colour: Option<egui::Color32>,
    ) -> Option<Target> {
        self.lay(ui, spans, font, colour, true)
    }

    /// A table cell: one line that does not wrap, because the table scrolls sideways
    /// instead. Wrapping inside a grid cell makes every column as narrow as its longest
    /// word — `Semester` comes out as `Semes` over `ter`.
    fn line(&self, ui: &mut Ui, spans: &[Span], colour: Option<egui::Color32>) -> Option<Target> {
        self.lay(ui, spans, None, colour, false)
    }

    /// Runs carry their own spaces, so the row adds none. `wrap` is the whole difference
    /// between a paragraph and a table cell: egui reads it off the layout, giving
    /// `TextWrapMode::Wrap` for a wrapping row and `Extend` for a plain one.
    fn lay(
        &self,
        ui: &mut Ui,
        spans: &[Span],
        font: Option<FontId>,
        colour: Option<egui::Color32>,
        wrap: bool,
    ) -> Option<Target> {
        let theme = self.theme;
        let mut followed = None;
        let mut row = |ui: &mut Ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for span in spans {
                let mut text = RichText::new(&span.text);
                if let Some(font) = &font {
                    text = text.font(font.clone());
                }
                if span.bold {
                    text = text.strong().color(color(theme.strong));
                }
                if span.italic {
                    text = text.italics();
                }
                if span.strike {
                    text = text.strikethrough().color(color(theme.muted));
                }
                if span.code {
                    text = text.monospace().color(color(theme.code));
                } else if let Some(colour) = colour {
                    text = text.color(colour);
                }
                // A link says so in the accent colour: the theme sets one text colour for the
                // whole window, so egui's own link colour never reaches it.
                let response = match &span.link {
                    Some(_) => ui.link(text.color(color(theme.accent))),
                    None => ui.label(text),
                };
                if let Some(hover) = &span.hover {
                    response.clone().on_hover_text(hover);
                }
                if response.clicked() {
                    followed = span.link.clone();
                }
            }
        };
        match wrap {
            true => ui.horizontal_wrapped(row),
            false => ui.horizontal(row),
        };
        followed
    }
}
