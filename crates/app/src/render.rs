//! Drawing a frame: the links, the discs, their icons and their names.
//!
//! The graph is painted on egui's background layer, across the whole window, and the
//! chrome is drawn over it — the same arrangement the canvas and the panels always had.
//! The lit set is the one mechanism that dims it: a legend filter holds it, and the click
//! or hover focus builds it from a node and its neighbours. Everything outside it draws
//! at 0.12 alpha.

use crate::emoji::Icons;
use crate::filter::{matches, Filter};
use crate::sim::{Sim, LABEL_PX};
use crate::theme::Theme;
use brain_map_model::Graph;
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke};
use std::collections::HashSet;

/// The whole of a texture, which is all any emoji picture is ever drawn from.
const WHOLE: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));

/// The disc is drawn once and the halo is a few rings around it, since a painter has no
/// radial gradient. Enough rings that the edge does not band, few enough to be free.
const HALO_RINGS: usize = 6;
const DIM: f32 = 0.12;

/// What stays bright. `None` means everything.
pub fn lit(sim: &Sim, filter: Option<&Filter>) -> Option<HashSet<usize>> {
    if filter.is_some() {
        return Some(
            sim.active
                .iter()
                .copied()
                .filter(|&i| matches(&sim.nodes[i], filter))
                .collect(),
        );
    }
    let focus = sim.selected.or(sim.hover)?;
    let mut set = HashSet::from([focus]);
    set.extend(
        sim.neighbours[focus]
            .iter()
            .copied()
            .filter(|&j| sim.born[j]),
    );
    Some(set)
}

pub struct Frame<'a> {
    pub painter: &'a Painter,
    /// The whole window. The explorer covers its left edge, which is what shifts the
    /// centre the camera aims at.
    pub rect: Rect,
    pub sim: &'a Sim,
    pub graph: &'a Graph,
    pub theme: &'a Theme,
    pub lit: Option<HashSet<usize>>,
}

impl Frame<'_> {
    /// A world position on the screen. The camera is the whole transform: there is no
    /// painter state to push and pop.
    fn at(&self, x: f64, y: f64) -> Pos2 {
        let view = &self.sim.view;
        let centre = (
            self.rect.left() as f64 + self.sim.viewport.w / 2.0,
            self.rect.top() as f64 + self.sim.viewport.h / 2.0,
        );
        Pos2::new(
            (centre.0 + (x - view.x) * view.k) as f32,
            (centre.1 + (y - view.y) * view.k) as f32,
        )
    }

    pub fn draw(&self, icons: &mut Icons) {
        let frame = self;
        let (painter, sim, theme) = (frame.painter, frame.sim, frame.theme);
        painter.rect_filled(frame.rect, 0.0, crate::theme::color(theme.canvas_bg));

        let lit = frame.lit.as_ref();
        for &li in &sim.active_links {
            let link = &sim.links[li];
            let on =
                lit.is_some_and(|set| set.contains(&link.source) && set.contains(&link.target));
            let colour = crate::theme::color(match (lit.is_some(), on) {
                (false, _) => theme.link,
                (true, true) => theme.link_lit,
                (true, false) => theme.link_dim,
            });
            painter.line_segment(
                [
                    frame.at(sim.nodes[link.source].x, sim.nodes[link.source].y),
                    frame.at(sim.nodes[link.target].x, sim.nodes[link.target].y),
                ],
                Stroke::new(0.6_f32, colour),
            );
        }

        let k = sim.view.k as f32;
        for &i in &sim.active {
            let node = &sim.nodes[i];
            let dim = lit.is_some_and(|set| !set.contains(&i));
            let colour = fade(theme.group_colour(&frame.graph.groups, &node.group), dim);
            let centre = frame.at(node.x, node.y);
            let radius = node.r as f32 * k;
            if node.glow > 0.0 && !dim {
                let reach = (node.r + node.glow / 3.0) as f32 * k;
                for ring in (1..=HALO_RINGS).rev() {
                    let at = ring as f32 / HALO_RINGS as f32;
                    painter.circle_filled(
                        centre,
                        radius + (reach - radius) * at,
                        colour.gamma_multiply(0.33 * (1.0 - at)),
                    );
                }
            }
            painter.circle_filled(centre, radius, colour);
        }

        // Every emoji was written in a note. Nothing here or in the scanner invents one,
        // so a node without an icon is simply a disc. Below this size a glyph is mush.
        for &i in &sim.active {
            let node = &sim.nodes[i];
            let icon = node.icon.as_str();
            if icon.is_empty() || sim.view.k * node.r < 7.0 {
                continue;
            }
            let dim = lit.is_some_and(|set| !set.contains(&i));
            let centre = frame.at(node.x, node.y);
            let tint = fade(Color32::WHITE, dim);
            match icons.texture(painter.ctx(), icon) {
                Some(texture) => {
                    let side = node.r as f32 * 1.6 * k;
                    painter.image(
                        texture.id(),
                        Rect::from_center_size(centre, eframe::egui::vec2(side, side)),
                        WHOLE,
                        tint,
                    );
                }
                // No colour font on this machine: draw the character and take the outline
                // egui can make of it.
                None => {
                    let size = node.r as f32 * 1.35 * k;
                    painter.text(
                        centre,
                        Align2::CENTER_CENTER,
                        icon,
                        FontId::proportional(size),
                        tint,
                    );
                }
            }
        }

        let label_colour = crate::theme::color(theme.label);
        for &i in &sim.active {
            let node = &sim.nodes[i];
            let show = match lit {
                Some(set) => set.contains(&i),
                None => sim.view.k * node.r > 8.0,
            };
            if show {
                let above = frame.at(node.x, node.y - node.r) - eframe::egui::vec2(0.0, 5.0);
                painter.text(
                    above,
                    Align2::CENTER_BOTTOM,
                    &node.label,
                    FontId::proportional(LABEL_PX as f32),
                    label_colour,
                );
            }
        }
    }
}

fn fade(colour: Color32, dim: bool) -> Color32 {
    match dim {
        true => colour.gamma_multiply(DIM),
        false => colour,
    }
}

/// What the bar counts, which is what the filter leaves lit when one is on.
pub fn count_label(sim: &Sim, filter: Option<&Filter>, lit: Option<&HashSet<usize>>) -> String {
    match (filter, lit) {
        (Some(filter), Some(lit)) => {
            format!(
                "{} of {} notes · {}",
                lit.len(),
                sim.active.len(),
                filter.key()
            )
        }
        _ => format!(
            "{} notes · {} links",
            sim.active.len(),
            sim.active_links.len()
        ),
    }
}
