//! What the window knows, and the actions that change it.
//!
//! [`Ui`] is what the person chose and what the chrome shows — the theme, the filter, the
//! note being read, the widths. [`Engine`] is the graph itself: the simulation, the
//! camera, the search, and the pointer acting on all three. Neither has public fields:
//! every change is a method that says what it is for, so there is one place to read to
//! know what can happen to a thing.

use crate::emoji::Icons;
use crate::filter::Filter;
use crate::render::{self, Frame};
use crate::settings::Settings;
use crate::sim::{Sim, LABEL_PX};
use crate::theme::{Theme, THEMES};
use crate::{links, tree};
use brain_map_model::{is_structural, Graph};
use eframe::egui::{Color32, Context, FontId, Painter, Pos2, Rect};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

const MIN_PANEL: f64 = 220.0;
pub const MAX_PANEL: f64 = 900.0;
/// How far a key moves the camera, in screen pixels, so it is the same at any zoom.
const PAN: f64 = 60.0;
/// A wheel notch is worth about this much zoom. Scroll deltas are points, not the
/// browser's lines, so it is not the number the page used to carry.
const ZOOM: f64 = 0.0025;
const MIN_ZOOM: f64 = 0.1;
const MAX_ZOOM: f64 = 8.0;

/// What the person chose, and what the chrome shows them.
pub struct Ui {
    settings: Settings,
    count: String,
    filter: Option<Filter>,
    /// The note being read, as a node index.
    reading: Option<usize>,
    full: bool,
    theme: usize,
    panel_w: f64,
    panel_hidden: bool,
    budget: u32,
    settings_open: bool,
    picker_open: bool,
    recent: Vec<String>,
    /// The explorer, as the keyboard sees it: the folders that are shut, the row the
    /// cursor is on, and whether it has been moved since the panel last scrolled to it.
    closed: HashSet<String>,
    cursor: Option<tree::Key>,
    chased: bool,
    /// Lines the reader was asked to scroll, taken by the note panel on the next frame.
    scroll: f32,
}

impl Ui {
    pub fn new(vault_chosen: bool) -> Ui {
        let settings = Settings::load();
        Ui {
            count: String::new(),
            filter: None,
            reading: None,
            full: false,
            theme: settings.theme().unwrap_or(0),
            panel_w: settings.panel(MIN_PANEL),
            panel_hidden: !settings.explorer(),
            budget: settings.budget(crate::sim::GROWTH_MS),
            settings_open: false,
            // No vault yet: the window opens on the picker instead of an empty graph. A
            // vault that happens to hold no notes is still a vault, so it is the path
            // that decides, not the count.
            picker_open: !vault_chosen,
            recent: settings.recent(),
            closed: HashSet::new(),
            cursor: None,
            chased: false,
            scroll: 0.0,
            settings,
        }
    }

    /// The settings a new engine inherits: what the person chose, not what they were
    /// looking at. A reload keeps the theme and the widths, never the open note.
    pub fn inherit(&mut self, from: &Ui) {
        self.theme = from.theme;
        self.panel_w = from.panel_w;
        self.panel_hidden = from.panel_hidden;
        self.budget = from.budget;
    }

    pub fn count(&self) -> &str {
        &self.count
    }

    pub fn filter(&self) -> Option<&Filter> {
        self.filter.as_ref()
    }

    pub fn reading(&self) -> Option<usize> {
        self.reading
    }

    /// Whether a folder is open. A folder nobody has shut is open, which is how the tree
    /// has always drawn itself.
    pub fn dir_open(&self, path: &str) -> bool {
        !self.closed.contains(path)
    }

    pub fn cursor(&self) -> Option<&tree::Key> {
        self.cursor.as_ref()
    }

    /// True once, on the frame after a motion: the panel scrolls the cursor into view and
    /// then leaves the scrollbar alone.
    pub fn take_chase(&mut self) -> bool {
        std::mem::take(&mut self.chased)
    }

    /// What the reader was asked to scroll, in points, taken by the note panel.
    pub fn take_scroll(&mut self) -> f32 {
        std::mem::take(&mut self.scroll)
    }

    /// The reader, scrolled by a key rather than a wheel.
    pub fn scroll_by(&mut self, points: f32) {
        self.scroll += points;
    }

    /// A row the pointer chose. The keyboard and the mouse share one cursor, so `j` after
    /// a click carries on from what was clicked.
    pub fn point_at(&mut self, key: tree::Key) {
        self.cursor = Some(key);
    }

    pub fn toggle_dir(&mut self, path: &str) {
        if !self.closed.remove(path) {
            self.closed.insert(path.to_string());
        }
    }

    pub fn is_full(&self) -> bool {
        self.full
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme]
    }

    pub fn theme_at(&self) -> usize {
        self.theme
    }

    pub fn panel_width(&self) -> f64 {
        self.panel_w
    }

    pub fn explorer_hidden(&self) -> bool {
        self.panel_hidden
    }

    pub fn budget(&self) -> u32 {
        self.budget
    }

    pub fn settings_open(&self) -> bool {
        self.settings_open
    }

    pub fn picker_open(&self) -> bool {
        self.picker_open
    }

    pub fn recent(&self) -> &[String] {
        &self.recent
    }

    /// Clicking a legend row keeps only what it names lit; clicking it again lets go.
    pub fn toggle_filter(&mut self, chosen: Filter) {
        self.filter = crate::filter::toggled(self.filter.as_ref(), chosen);
    }

    pub fn clear_filter(&mut self) {
        self.filter = None;
    }

    /// A full-width panel holding nothing would cover the graph with no way back, so
    /// closing the note drops fullscreen with it.
    pub fn stop_reading(&mut self) {
        self.reading = None;
        self.full = false;
    }

    /// Fullscreen refuses to act with no note open, for the same reason.
    pub fn toggle_full(&mut self) {
        if self.reading.is_some() {
            self.full = !self.full;
        }
    }

    pub fn set_theme(&mut self, at: usize) {
        if at < THEMES.len() {
            self.theme = at;
            self.settings.set_theme(THEMES[at].key);
        }
    }

    pub fn open_settings(&mut self) {
        self.settings_open = true;
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
    }

    pub fn open_picker(&mut self) {
        self.picker_open = true;
    }

    pub fn close_picker(&mut self) {
        self.picker_open = false;
    }

    pub fn remember_vault(&mut self, vault: &str) {
        self.recent = self.settings.remember(vault);
    }
}

pub struct Engine {
    graph: Graph,
    sim: Sim,
    index: links::Index,
    tree: tree::Dir,
    group_counts: Vec<usize>,
    signal_counts: Vec<(String, usize)>,
    top_tags: Vec<(String, usize)>,
    found: Vec<usize>,
    found_at: usize,
    /// The node the pointer is holding, if it is holding one.
    dragging: Option<usize>,
    panning: bool,
    ui: Ui,
}

impl Engine {
    pub fn new(graph: Graph, ui: Ui, ctx: &Context) -> Engine {
        // Names are what crowd the graph, so each is measured once, by the same fonts
        // that will draw it.
        let sim = Sim::new(&graph, |label| measure(ctx, label), seed());
        let ids: Vec<String> = graph.nodes.iter().map(|n| n.id.clone()).collect();
        let group_keys: Vec<String> = graph.groups.iter().map(|g| g.key.clone()).collect();
        Engine {
            index: links::Index::of(ids.clone()),
            tree: tree::build(&ids),
            group_counts: crate::filter::group_counts(&sim.nodes, &group_keys),
            signal_counts: crate::filter::signal_counts(&sim.nodes),
            top_tags: crate::filter::top_tags(&sim.nodes, 12),
            sim,
            graph,
            found: Vec::new(),
            found_at: usize::MAX,
            dragging: None,
            panning: false,
            ui,
        }
    }

    pub fn ui(&self) -> &Ui {
        &self.ui
    }

    pub fn ui_mut(&mut self) -> &mut Ui {
        &mut self.ui
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn tree(&self) -> &tree::Dir {
        &self.tree
    }

    /// The explorer's rows as the keyboard walks them: the tree flattened to what is
    /// drawn, so the cursor cannot land on a row inside a folder that is shut.
    pub fn rows(&self) -> Vec<tree::Row> {
        tree::rows(&self.tree, &|path| !self.ui.dir_open(path))
    }

    fn cursor_at(&self, rows: &[tree::Row]) -> Option<usize> {
        let key = self.ui.cursor()?;
        rows.iter().position(|row| &row.key == key)
    }

    fn put_cursor(&mut self, key: tree::Key) {
        self.ui.point_at(key);
        self.ui.chased = true;
    }

    /// `j` and `k`. The cursor stops at either end rather than wrapping: a tree is a list
    /// you are looking down, not a ring.
    pub fn move_cursor(&mut self, step: isize) {
        let rows = self.rows();
        if rows.is_empty() {
            return;
        }
        let at = match self.cursor_at(&rows) {
            Some(at) => (at as isize + step).clamp(0, rows.len() as isize - 1) as usize,
            None if step > 0 => 0,
            None => rows.len() - 1,
        };
        self.put_cursor(rows[at].key.clone());
    }

    /// `g` and `G`.
    pub fn cursor_edge(&mut self, end: bool) {
        let rows = self.rows();
        let Some(row) = (match end {
            true => rows.last(),
            false => rows.first(),
        }) else {
            return;
        };
        let key = row.key.clone();
        self.put_cursor(key);
    }

    /// `h`: shut the folder you are on, or step out to the one you are in.
    pub fn cursor_out(&mut self) {
        let rows = self.rows();
        let Some(at) = self.cursor_at(&rows) else {
            return self.move_cursor(1);
        };
        if let Some(path) = rows[at].dir().filter(|path| self.ui.dir_open(path)) {
            let path = path.to_string();
            self.ui.toggle_dir(&path);
            return;
        }
        if let Some(parent) = tree::parent(&rows, at) {
            self.put_cursor(rows[parent].key.clone());
        }
    }

    /// `l` and Enter: open the folder, step into the one already open, or read the note.
    pub fn cursor_into(&mut self) {
        let rows = self.rows();
        let Some(at) = self.cursor_at(&rows) else {
            return self.move_cursor(1);
        };
        if let Some(node) = rows[at].file() {
            self.go_to(node);
            return;
        }
        let Some(path) = rows[at].dir() else { return };
        if !self.ui.dir_open(path) {
            let path = path.to_string();
            self.ui.toggle_dir(&path);
            return;
        }
        // Already open: the next row is what is inside it, if it holds anything.
        if let Some(row) = rows.get(at + 1).filter(|row| row.depth > rows[at].depth) {
            self.put_cursor(row.key.clone());
        }
    }

    pub fn group_counts(&self) -> &[usize] {
        &self.group_counts
    }

    pub fn signal_counts(&self) -> &[(String, usize)] {
        &self.signal_counts
    }

    pub fn top_tags(&self) -> &[(String, usize)] {
        &self.top_tags
    }

    /// One node, as the chrome asks about it: its id, its name and its tags.
    pub fn node(&self, at: usize) -> &crate::sim::Node {
        &self.sim.nodes[at]
    }

    /// What the concept's frontmatter said about itself, for the reader to show. The
    /// graph and the simulation index nodes the same way, so it is the same `at`.
    pub fn concept(&self, at: usize) -> &brain_map_model::Concept {
        &self.graph.nodes[at].concept
    }

    /// The notes whose text links to this one — the graph's edges, read backwards.
    pub fn cited_by(&self, at: usize) -> Vec<usize> {
        let mut from: Vec<usize> = self
            .sim
            .links
            .iter()
            .filter(|l| l.target == at && !is_structural(&self.sim.nodes[l.source].id))
            .map(|l| l.source)
            .collect();
        from.sort_unstable();
        from.dedup();
        from
    }

    pub fn node_id(&self, at: usize) -> &str {
        &self.sim.nodes[at].id
    }

    /// A node's emoji, which is whatever its note declared.
    pub fn icon_of(&self, at: usize) -> &str {
        &self.sim.nodes[at].icon
    }

    pub fn find(&self, id: &str) -> Option<usize> {
        self.graph.nodes.iter().position(|n| n.id == id)
    }

    /// The window's size. Every resize goes through the panel width too: a window that
    /// shrank may no longer have room for the width the person chose.
    pub fn resize(&mut self, w: f64, h: f64) {
        self.sim.viewport.w = w;
        self.sim.viewport.h = h;
        self.set_panel(self.ui.panel_w);
    }

    /// The explorer's width lives in one place, and this is the only writer: the panel is
    /// laid out against it. The graph does not move for it — it stays centred on the
    /// window and the explorer floats over its left edge.
    pub fn set_panel(&mut self, px: f64) {
        let room = (self.sim.viewport.w - 200.0).max(MIN_PANEL);
        self.ui.panel_w = px.clamp(MIN_PANEL, room.min(MAX_PANEL));
    }

    pub fn hide_panel(&mut self, on: bool) {
        self.ui.panel_hidden = on;
        self.set_panel(self.ui.panel_w);
        self.remember_panel();
    }

    /// The explorer as it was left, written to the settings file. A drag moves the width
    /// every frame, so this is called when it is let go rather than while it moves.
    pub fn remember_panel(&mut self) {
        let (width, hidden) = (self.ui.panel_w, self.ui.panel_hidden);
        self.ui.settings.set_panel(width, hidden);
    }

    pub fn frame(&mut self, delta_ms: f64) {
        self.sim.frame(delta_ms);
        self.sim.camera_step();
    }

    pub fn draw(&mut self, painter: &Painter, rect: Rect, icons: &mut Icons) {
        let lit = render::lit(&self.sim, self.ui.filter.as_ref());
        self.ui.count = render::count_label(&self.sim, self.ui.filter.as_ref(), lit.as_ref());
        Frame {
            painter,
            rect,
            sim: &self.sim,
            graph: &self.graph,
            theme: self.ui.theme(),
            lit,
        }
        .draw(icons);
    }

    /// Clicking a note opens its source next to the graph; folder and vault nodes have
    /// none, so they close whatever was open instead.
    pub fn open(&mut self, node: Option<usize>) {
        match node.filter(|&i| !is_structural(&self.sim.nodes[i].id)) {
            Some(i) => {
                self.hide_panel(false);
                self.ui.reading = Some(i);
            }
            None => self.ui.stop_reading(),
        }
    }

    /// Drop the selection, the filter and the open note — everything Esc lets go of.
    pub fn release(&mut self) {
        self.sim.selected = None;
        self.ui.clear_filter();
        self.ui.stop_reading();
    }

    pub fn close_note(&mut self) {
        self.sim.selected = None;
        self.ui.stop_reading();
    }

    pub fn go_to(&mut self, node: usize) {
        self.sim.selected = Some(node);
        self.sim.user_cam = true;
        self.sim.view.x = self.sim.nodes[node].x;
        self.sim.view.y = self.sim.nodes[node].y;
        self.sim.view.k = self.sim.view.k.max(2.2);
        self.open(Some(node));
    }

    pub fn search(&mut self, query: &str) {
        self.found = self.sim.search(query);
        self.found_at = usize::MAX;
        self.step(1);
    }

    /// Walks the matches, wrapping at either end.
    pub fn step(&mut self, direction: isize) {
        if self.found.is_empty() {
            return;
        }
        let len = self.found.len();
        self.found_at = match self.found_at {
            usize::MAX if direction > 0 => 0,
            usize::MAX => len - 1,
            at => ((at as isize + direction).rem_euclid(len as isize)) as usize,
        };
        self.go_to(self.found[self.found_at]);
    }

    pub fn restart(&mut self) {
        self.sim.restart();
        self.found.clear();
        self.found_at = usize::MAX;
        self.release();
    }

    /// A budget you cannot see applied is a budget you cannot judge, so changing it
    /// rebuilds the schedule and replays. Rebuilding is from scratch: asking twice for the
    /// same budget lands in the same place rather than compounding.
    pub fn set_budget(&mut self, ms: u32) {
        self.ui.budget = ms;
        self.ui.settings.set_budget(ms);
        self.schedule(ms);
        self.restart();
    }

    pub fn schedule(&mut self, budget_ms: u32) {
        let graph = std::mem::take(&mut self.graph);
        self.sim.schedule(&graph, budget_ms);
        self.graph = graph;
    }

    // --- the pointer, which is the only thing that moves the camera by hand ---

    /// Zoom about a point, which stays where it is under the cursor.
    pub fn zoom(&mut self, at: Pos2, notches: f64) {
        self.sim.user_cam = true;
        let (x, y) = (at.x as f64, at.y as f64);
        let (wx, wy) = self.sim.to_world(x, y);
        let k = (self.sim.view.k * (notches * ZOOM).exp()).clamp(MIN_ZOOM, MAX_ZOOM);
        let viewport = self.sim.viewport;
        self.sim.view.k = k;
        self.sim.view.x = wx - (x - viewport.w / 2.0) / k;
        self.sim.view.y = wy - (y - viewport.h / 2.0) / k;
    }

    /// Drag the camera by a screen distance.
    pub fn pan_by(&mut self, dx: f32, dy: f32) {
        let k = self.sim.view.k;
        self.sim.view.x -= dx as f64 / k;
        self.sim.view.y -= dy as f64 / k;
    }

    /// One key press worth of camera, in the same direction at any zoom.
    pub fn pan_step(&mut self, dx: f64, dy: f64) {
        self.sim.user_cam = true;
        let k = self.sim.view.k;
        self.sim.view.x += dx * PAN / k;
        self.sim.view.y += dy * PAN / k;
    }

    /// The pointer went down: on a node it pins it, on empty space it takes the camera.
    pub fn grab(&mut self, at: Pos2) {
        self.dragging = self.sim.hit(at.x as f64, at.y as f64);
        self.panning = self.dragging.is_none();
        match self.dragging {
            Some(node) => {
                self.sim.nodes[node].fx = Some(self.sim.nodes[node].x);
                self.sim.nodes[node].fy = Some(self.sim.nodes[node].y);
                self.sim.alpha = self.sim.alpha.max(0.2);
            }
            None => self.sim.user_cam = true,
        }
    }

    pub fn drag_to(&mut self, at: Pos2) {
        let Some(node) = self.dragging else { return };
        let (wx, wy) = self.sim.to_world(at.x as f64, at.y as f64);
        self.sim.nodes[node].fx = Some(wx);
        self.sim.nodes[node].fy = Some(wy);
        self.sim.alpha = self.sim.alpha.max(0.15);
    }

    pub fn dragging_node(&self) -> bool {
        self.dragging.is_some()
    }

    pub fn let_go(&mut self) {
        if let Some(node) = self.dragging.take() {
            self.sim.nodes[node].fx = None;
            self.sim.nodes[node].fy = None;
        }
        self.panning = false;
    }

    /// A click selects a node, or lets go of the one that was selected.
    pub fn click(&mut self, at: Pos2) {
        let hit = self.sim.hit(at.x as f64, at.y as f64);
        self.sim.selected = match hit == self.sim.selected {
            true => None,
            false => hit,
        };
        let selected = self.sim.selected;
        self.open(selected);
    }

    pub fn hover(&mut self, at: Option<Pos2>) {
        self.sim.hover = match (self.dragging, at) {
            (None, Some(at)) => self.sim.hit(at.x as f64, at.y as f64),
            _ => None,
        };
    }

    /// The link a note points at, resolved the way the graph resolved it into an edge.
    pub fn resolve(&self, target: &str, from: Option<&str>, relative: bool) -> Option<usize> {
        self.index.resolve(target, from, relative)
    }
}

/// How wide a name is drawn, asked of the fonts that will draw it.
fn measure(ctx: &Context, label: &str) -> f64 {
    ctx.fonts(|fonts| {
        fonts
            .layout_no_wrap(
                label.to_string(),
                FontId::proportional(LABEL_PX as f32),
                Color32::WHITE,
            )
            .size()
            .x as f64
    })
}

/// The layout is deterministic given a seed, and a new one each run keeps two openings of
/// the same vault from looking identical.
fn seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0x9e37_79b9_7f4a_7c15, |d| {
            d.as_nanos() as u64 ^ 0x9e37_79b9_7f4a_7c15
        })
}
