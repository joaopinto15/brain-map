//! The graph the page moves: the node and link model, the growth schedule, and the
//! hand-rolled port of d3-force that settles it.
//!
//! Nothing here touches the DOM. The canvas measures its own labels and hands the widths
//! in, which is what lets `cargo test` run the whole simulation on the host.

use brain_map_model::Graph;

pub const LINK_DISTANCE: f64 = 30.0;
pub const LINK_STRENGTH: f64 = 0.5;
const CHARGE_MAX2: f64 = 450.0 * 450.0;
/// A link that leaves its folder rests this much longer, so the sections drift into
/// islands with visible air between them instead of one even mesh.
pub const GROUP_GAP: f64 = 2.2;
/// Labels are wide and short. Counting vertical distance this much heavier turns the
/// collide circle into a lozenge the shape of a name, so nodes may stack closely above
/// one another while never sitting side by side with their labels overlapping.
pub const LABEL_SQUASH: f64 = 2.5;
const CENTER_STRENGTH: f64 = 0.05;
const GRAVITY: f64 = 0.03;
pub const COLLIDE_PAD: f64 = 3.5;
const COLLIDE_STRENGTH: f64 = 0.9;
const VELOCITY_DECAY: f64 = 0.6;
const ALPHA_DECAY: f64 = 0.012;
pub const ALPHA_FLOOR: f64 = 0.004;

/// A beat before the first node, so the canvas is not born mid-frame.
pub const GROWTH_START: f64 = 600.0;
/// How long the whole vault may take to assemble, by default.
pub const GROWTH_MS: u32 = 8000;
pub const LABEL_PX: f64 = 11.0;
/// Capped, so one very long filename cannot open a crater the layout works around.
const LABEL_HW_MAX: f64 = 70.0;

pub struct Node {
    pub id: String,
    pub label: String,
    pub group: String,
    pub tags: Vec<String>,
    /// The emoji the note declared, or empty. Nothing here derives one.
    pub icon: String,
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub fx: Option<f64>,
    pub fy: Option<f64>,
    pub r: f64,
    /// Half the width of the name drawn above the node, which is what the collide pass
    /// actually has to keep apart.
    pub hw: f64,
    pub charge: f64,
    /// When this node joins the graph, in milliseconds from the start of the growth.
    pub t: f64,
    pub glow: f64,
}

pub struct Link {
    pub source: usize,
    pub target: usize,
    pub bias: f64,
    /// Leaves its own folder, decided once here rather than told apart mid-tick.
    pub far: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub x: f64,
    pub y: f64,
    pub k: f64,
}

/// The window the graph is drawn into, and how much of it the explorer covers.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub w: f64,
    pub h: f64,
    pub panel: f64,
}

pub struct Sim {
    pub nodes: Vec<Node>,
    pub links: Vec<Link>,
    pub neighbours: Vec<Vec<usize>>,
    /// Node indices in the order they were born; hit testing walks it backwards.
    pub active: Vec<usize>,
    pub active_links: Vec<usize>,
    pub born: Vec<bool>,
    pub next_idx: usize,
    pub alpha: f64,
    pub clock: f64,
    pub done: bool,
    pub view: View,
    pub user_cam: bool,
    pub selected: Option<usize>,
    pub hover: Option<usize>,
    pub viewport: Viewport,
    seed: u64,
}

impl Sim {
    /// `label_width` measures a name the way the canvas will draw it.
    pub fn new(graph: &Graph, label_width: impl Fn(&str) -> f64, seed: u64) -> Sim {
        let mut degree = vec![0usize; graph.nodes.len()];
        for l in &graph.links {
            degree[l.s] += 1;
            degree[l.t] += 1;
        }
        let nodes: Vec<Node> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let group = graph.group(&n.group);
                let radius = group.map_or(5.0, |g| g.radius);
                let r = (radius * 2.2).min(radius * (0.8 + (degree[i] as f64).sqrt() * 0.15));
                Node {
                    id: n.id.clone(),
                    label: n.label.clone(),
                    group: n.group.clone(),
                    tags: n.tags.clone(),
                    icon: n.icon.clone(),
                    x: 0.0,
                    y: 0.0,
                    vx: 0.0,
                    vy: 0.0,
                    fx: None,
                    fy: None,
                    r,
                    hw: (label_width(&n.label) / 2.0).max(r).min(LABEL_HW_MAX),
                    charge: -(15.0 + r * 6.0),
                    t: 0.0,
                    // A node's colour is the theme's to decide and can change under a
                    // settled graph, so the renderer looks it up rather than keeping it.
                    glow: group.map_or(0.0, |g| g.glow),
                }
            })
            .collect();
        let links: Vec<Link> = graph
            .links
            .iter()
            .map(|l| {
                let count = degree[l.s] + degree[l.t];
                Link {
                    source: l.s,
                    target: l.t,
                    bias: match count {
                        0 => 0.5,
                        _ => degree[l.s] as f64 / count as f64,
                    },
                    far: nodes[l.s].group != nodes[l.t].group,
                }
            })
            .collect();
        let mut neighbours = vec![Vec::new(); nodes.len()];
        for l in &links {
            neighbours[l.source].push(l.target);
            neighbours[l.target].push(l.source);
        }
        let born = vec![false; nodes.len()];
        let mut sim = Sim {
            nodes,
            links,
            neighbours,
            active: Vec::new(),
            active_links: Vec::new(),
            born,
            next_idx: 0,
            alpha: 0.0,
            clock: 0.0,
            done: false,
            view: View {
                x: 0.0,
                y: 0.0,
                k: 1.5,
            },
            user_cam: false,
            selected: None,
            hover: None,
            viewport: Viewport {
                w: 1280.0,
                h: 800.0,
                panel: 0.0,
            },
            seed: seed | 1,
        };
        sim.schedule(graph, GROWTH_MS);
        sim
    }

    /// Each group enters as a burst, paced by its own size. Left alone the schedule runs
    /// longer the more groups a vault has — a folder costs its pause whether it holds
    /// three notes or three hundred — so a finished schedule longer than the budget is
    /// squeezed into it, keeping the relative pacing and bounding the wait.
    pub fn schedule(&mut self, graph: &Graph, budget_ms: u32) {
        let mut clock_at = GROWTH_START;
        let mut prev_group: Option<&str> = None;
        for node in &mut self.nodes {
            let group = graph.group(&node.group);
            if prev_group != Some(node.group.as_str()) {
                clock_at += group.map_or(500.0, |g| g.pause as f64);
                prev_group = Some(&node.group);
            }
            clock_at += group.map_or(60.0, |g| g.pace as f64);
            node.t = clock_at;
        }
        let span = clock_at - GROWTH_START;
        if span > budget_ms as f64 {
            let squeeze = budget_ms as f64 / span;
            for node in &mut self.nodes {
                node.t = GROWTH_START + (node.t - GROWTH_START) * squeeze;
            }
        }
    }

    /// Back to one node and no motion, keeping the schedule the budget last built.
    pub fn restart(&mut self) {
        for n in &mut self.nodes {
            n.vx = 0.0;
            n.vy = 0.0;
            n.fx = None;
            n.fy = None;
        }
        self.active.clear();
        self.active_links.clear();
        self.born.iter_mut().for_each(|b| *b = false);
        self.next_idx = 0;
        self.clock = 0.0;
        self.alpha = 0.0;
        self.done = false;
        self.selected = None;
        self.hover = None;
        self.user_cam = false;
        self.view = View {
            x: 0.0,
            y: 0.0,
            k: 1.5,
        };
    }

    /// Deterministic, so a layout is reproducible and the tests are stable.
    fn random(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Everything the schedule says is due by `time` joins the graph.
    pub fn activate(&mut self, time: f64) {
        let mut changed = false;
        while self.next_idx < self.nodes.len() && self.nodes[self.next_idx].t <= time {
            let i = self.next_idx;
            let anchor = self.neighbours[i].iter().copied().find(|&j| self.born[j]);
            let angle = self.random() * 7.0;
            // Born clear of what it lands next to. Inside the collide radius it would be
            // flung out on its first tick, which is the jolt you see rather than a node
            // growing in.
            let away = match anchor {
                Some(j) => self.nodes[j].hw + self.nodes[i].hw + COLLIDE_PAD,
                None => 30.0,
            };
            let (ax, ay) = anchor.map_or((0.0, 0.0), |j| (self.nodes[j].x, self.nodes[j].y));
            let n = &mut self.nodes[i];
            n.x = ax + angle.cos() * away;
            n.y = ay + angle.sin() * away;
            n.vx = 0.0;
            n.vy = 0.0;
            self.active.push(i);
            self.born[i] = true;
            self.next_idx += 1;
            changed = true;
        }
        if changed {
            self.active_links = (0..self.links.len())
                .filter(|&l| self.born[self.links[l].source] && self.born[self.links[l].target])
                .collect();
            self.alpha = self
                .alpha
                .max((0.2 + self.active.len() as f64 / 800.0).min(0.5));
        }
    }

    // ponytail: O(n²) pair loop, smooth to a few thousand notes; Barnes-Hut past that.
    pub fn tick(&mut self) {
        self.alpha -= self.alpha * ALPHA_DECAY;
        let alpha = self.alpha;

        for &li in &self.active_links {
            let l = &self.links[li];
            let (s, t) = (l.source, l.target);
            let (bias, far) = (l.bias, l.far);
            let mut dx = self.nodes[t].x + self.nodes[t].vx - self.nodes[s].x - self.nodes[s].vx;
            let mut dy = self.nodes[t].y + self.nodes[t].vy - self.nodes[s].y - self.nodes[s].vy;
            let d = dx.hypot(dy).max(1e-6);
            let rest = match far {
                true => LINK_DISTANCE * GROUP_GAP,
                false => LINK_DISTANCE,
            };
            let w = (d - rest) / d * alpha * LINK_STRENGTH;
            dx *= w;
            dy *= w;
            self.nodes[t].vx -= dx * bias;
            self.nodes[t].vy -= dy * bias;
            self.nodes[s].vx += dx * (1.0 - bias);
            self.nodes[s].vy += dy * (1.0 - bias);
        }

        for ai in 0..self.active.len() {
            let a = self.active[ai];
            for bi in (ai + 1)..self.active.len() {
                let b = self.active[bi];
                let dx = self.nodes[b].x - self.nodes[a].x;
                let dy = self.nodes[b].y - self.nodes[a].y;
                // Coincident nodes would divide by zero and fling each other off-screen.
                let d2 = (dx * dx + dy * dy).max(1.0);
                if d2 < CHARGE_MAX2 {
                    let (qa, qb) = (self.nodes[a].charge, self.nodes[b].charge);
                    self.nodes[a].vx += dx * qb * alpha / d2;
                    self.nodes[a].vy += dy * qb * alpha / d2;
                    self.nodes[b].vx -= dx * qa * alpha / d2;
                    self.nodes[b].vy -= dy * qa * alpha / d2;
                }
                // `hw` is the node's own half-width including its label, so this keeps
                // names apart, not just discs. The plain distance is only the cheap
                // prefilter — it can never be smaller than the squashed one, so nothing
                // that touches is skipped.
                let touch = self.nodes[a].hw + self.nodes[b].hw + COLLIDE_PAD * 2.0;
                if d2 < touch * touch {
                    let mut cx =
                        self.nodes[b].x + self.nodes[b].vx - self.nodes[a].x - self.nodes[a].vx;
                    let mut cy =
                        (self.nodes[b].y + self.nodes[b].vy - self.nodes[a].y - self.nodes[a].vy)
                            * LABEL_SQUASH;
                    let d = cx.hypot(cy).max(1e-6);
                    if d < touch {
                        let push = (touch - d) / d * COLLIDE_STRENGTH;
                        cx *= push;
                        cy *= push / LABEL_SQUASH;
                        let (ra, rb) = (self.nodes[a].r, self.nodes[b].r);
                        let share = (rb * rb) / (ra * ra + rb * rb);
                        self.nodes[b].vx += cx * share;
                        self.nodes[b].vy += cy * share;
                        self.nodes[a].vx -= cx * (1.0 - share);
                        self.nodes[a].vy -= cy * (1.0 - share);
                    }
                }
            }
            self.nodes[a].vx -= self.nodes[a].x * GRAVITY * alpha;
            self.nodes[a].vy -= self.nodes[a].y * GRAVITY * alpha;
        }

        let (mut cx, mut cy) = (0.0, 0.0);
        for &i in &self.active {
            cx += self.nodes[i].x;
            cy += self.nodes[i].y;
        }
        let count = self.active.len().max(1) as f64;
        cx = cx / count * CENTER_STRENGTH;
        cy = cy / count * CENTER_STRENGTH;

        for &i in &self.active {
            let n = &mut self.nodes[i];
            match n.fx {
                None => {
                    n.vx *= VELOCITY_DECAY;
                    n.x += n.vx - cx;
                }
                Some(fx) => {
                    n.x = fx;
                    n.vx = 0.0;
                }
            }
            match n.fy {
                None => {
                    n.vy *= VELOCITY_DECAY;
                    n.y += n.vy - cy;
                }
                Some(fy) => {
                    n.y = fy;
                    n.vy = 0.0;
                }
            }
        }
    }

    /// One frame of clock. Returns whether anything moved that needs drawing.
    pub fn frame(&mut self, delta_ms: f64) {
        self.clock += delta_ms;
        if !self.done {
            self.activate(self.clock);
            if self.next_idx >= self.nodes.len() {
                self.done = true;
            }
        }
        if self.alpha > ALPHA_FLOOR {
            self.tick();
        }
    }

    /// The camera eases towards a frame that holds the whole graph, until the user takes
    /// over. The explorer never goes away, so the centre is the middle of what is left.
    pub fn camera_step(&mut self) {
        if self.active.is_empty() || self.user_cam {
            return;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &i in &self.active {
            x0 = x0.min(self.nodes[i].x);
            x1 = x1.max(self.nodes[i].x);
            y0 = y0.min(self.nodes[i].y);
            y1 = y1.max(self.nodes[i].y);
        }
        let fit = 2.0f64
            .min((self.viewport.w - self.viewport.panel) / (x1 - x0 + 220.0))
            .min(self.viewport.h / (y1 - y0 + 220.0));
        self.view.k += (fit - self.view.k) * 0.05;
        self.view.x += ((x0 + x1) / 2.0 - self.view.x) * 0.07;
        self.view.y += ((y0 + y1) / 2.0 - self.view.y) * 0.07;
    }

    pub fn to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        (
            (sx - (self.viewport.w + self.viewport.panel) / 2.0) / self.view.k + self.view.x,
            (sy - self.viewport.h / 2.0) / self.view.k + self.view.y,
        )
    }

    /// The topmost node under a screen point, if any.
    pub fn hit(&self, sx: f64, sy: f64) -> Option<usize> {
        let (wx, wy) = self.to_world(sx, sy);
        self.active.iter().rev().copied().find(|&i| {
            let n = &self.nodes[i];
            let r = n.r.max(5.0 / self.view.k) + 3.0 / self.view.k;
            (n.x - wx).powi(2) + (n.y - wy).powi(2) <= r * r
        })
    }

    /// Names first, then paths: a note whose name matches comes before one that only
    /// matches by where it lives.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return Vec::new();
        }
        let by_label: Vec<usize> = self
            .active
            .iter()
            .copied()
            .filter(|&i| self.nodes[i].label.to_lowercase().contains(&needle))
            .collect();
        let by_path = self.active.iter().copied().filter(|&i| {
            !by_label.contains(&i) && self.nodes[i].id.to_lowercase().contains(&needle)
        });
        by_label.iter().copied().chain(by_path).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brain_map_model::{Group, Link as GraphLink, Node as GraphNode};

    fn group(key: &str, pace: u32) -> Group {
        Group {
            key: key.into(),
            color: "#60a5fa".into(),
            radius: 6.0,
            glow: 0.0,
            name: key.into(),
            pace,
            pause: 450,
            major: true,
            cluster: false,
        }
    }

    /// The shape a vault actually has: a vault node over one folder node per top-level
    /// folder, every note hanging off its folder, and chords between nearby notes.
    fn vault(per_group: usize) -> Graph {
        let mut graph = Graph {
            groups: vec![
                Group {
                    radius: 11.0,
                    glow: 30.0,
                    pace: 0,
                    pause: 0,
                    ..group("router", 0)
                },
                group("ideas", 200),
                group("daily", 200),
            ],
            ..Graph::default()
        };
        graph.nodes.push(GraphNode {
            id: "__vault__".into(),
            label: "vault".into(),
            group: "router".into(),
            tags: vec![],
            icon: String::new(),
        });
        for g in ["ideas", "daily"] {
            graph.nodes.push(GraphNode {
                id: format!("__dir__{g}"),
                label: g.into(),
                group: g.into(),
                tags: vec![],
                icon: String::new(),
            });
            let dir = graph.nodes.len() - 1;
            graph.links.push(GraphLink { s: 0, t: dir });
            for i in 0..per_group {
                graph.nodes.push(GraphNode {
                    id: format!("{g}/{i}.md"),
                    label: format!("{g}-{i}"),
                    group: g.into(),
                    tags: match i % 5 {
                        0 => vec!["research".into(), format!("tag-{}", i % 3)],
                        _ => vec![],
                    },
                    icon: String::new(),
                });
                let n = graph.nodes.len() - 1;
                graph.links.push(GraphLink { s: dir, t: n });
                if i > 2 {
                    graph.links.push(GraphLink { s: n, t: n - 3 });
                }
            }
        }
        graph
    }

    /// The canvas measures names in pixels; on the host a name is 6px a character.
    fn measured(graph: &Graph) -> Sim {
        Sim::new(graph, |label| label.chars().count() as f64 * 6.0, 42)
    }

    fn settle(sim: &mut Sim) -> usize {
        for frame in 0..4000 {
            sim.frame(16.0);
            sim.camera_step();
            if sim.done && sim.alpha <= ALPHA_FLOOR {
                return frame;
            }
        }
        panic!("never settled");
    }

    #[test]
    fn the_growth_finishes_inside_its_budget() {
        let graph = vault(40);
        let mut sim = measured(&graph);
        sim.schedule(&graph, 3000);
        let last = sim.nodes.last().expect("nodes").t;
        assert!(last <= GROWTH_START + 3000.0, "last node at {last}ms");

        // Instant means instant: everything is due before the first frame lands.
        sim.schedule(&graph, 0);
        assert!(sim.nodes.iter().all(|n| n.t <= GROWTH_START));
    }

    #[test]
    fn every_node_arrives_and_the_graph_comes_to_rest() {
        let graph = vault(40);
        let mut sim = measured(&graph);
        settle(&mut sim);
        assert_eq!(sim.active.len(), sim.nodes.len(), "every node was born");
        assert!(sim.alpha <= ALPHA_FLOOR, "the simulation cooled");

        // Cooled means still: one more frame moves nothing the eye could catch.
        let before: Vec<(f64, f64)> = sim.nodes.iter().map(|n| (n.x, n.y)).collect();
        sim.frame(16.0);
        let drift = sim
            .nodes
            .iter()
            .zip(&before)
            .map(|(n, (x, y))| (n.x - x).hypot(n.y - y))
            .fold(0.0, f64::max);
        assert!(drift < 0.05, "settled graph drifted {drift}px in one frame");
    }

    #[test]
    fn settled_names_do_not_overlap() {
        let graph = vault(40);
        let mut sim = measured(&graph);
        settle(&mut sim);
        let mut worst: f64 = 0.0;
        for (ai, &a) in sim.active.iter().enumerate() {
            for &b in sim.active.iter().skip(ai + 1) {
                let (na, nb) = (&sim.nodes[a], &sim.nodes[b]);
                // The same lozenge the collide pass keeps apart: names are wide and short.
                let dx = (nb.x - na.x).abs();
                let dy = (nb.y - na.y).abs() * LABEL_SQUASH;
                let touch = na.hw + nb.hw;
                worst = worst.max(touch - dx.hypot(dy));
            }
        }
        assert!(worst < COLLIDE_PAD * 2.0, "names overlap by {worst}px");
    }

    #[test]
    fn folders_sit_further_apart_than_their_own_notes() {
        let graph = vault(40);
        let mut sim = measured(&graph);
        settle(&mut sim);
        let span = |far: bool| {
            let picked: Vec<f64> = sim
                .links
                .iter()
                .filter(|l| l.far == far)
                .map(|l| {
                    let (s, t) = (&sim.nodes[l.source], &sim.nodes[l.target]);
                    (t.x - s.x).hypot(t.y - s.y)
                })
                .collect();
            picked.iter().sum::<f64>() / picked.len() as f64
        };
        let (across, inside) = (span(true), span(false));
        assert!(
            across > inside * 1.5,
            "links across folders rest {across:.0}px, inside {inside:.0}px"
        );
    }

    #[test]
    fn the_camera_centres_on_the_window_left_of_the_explorer() {
        let graph = vault(20);
        let mut sim = measured(&graph);
        sim.viewport = Viewport {
            w: 1600.0,
            h: 900.0,
            panel: 420.0,
        };
        settle(&mut sim);
        let (cx, cy) = sim.to_world((1600.0 + 420.0) / 2.0, 900.0 / 2.0);
        assert!(
            (cx - sim.view.x).abs() < 1e-9,
            "{cx} is not the view centre"
        );
        assert!(
            (cy - sim.view.y).abs() < 1e-9,
            "{cy} is not the view centre"
        );
    }

    #[test]
    fn most_names_are_still_drawn_at_the_settled_zoom() {
        let graph = vault(40);
        let mut sim = measured(&graph);
        sim.viewport = Viewport {
            w: 1600.0,
            h: 900.0,
            panel: 420.0,
        };
        settle(&mut sim);
        // `render` draws a name when the node is bigger than 8 screen pixels.
        let readable = sim
            .active
            .iter()
            .filter(|&&i| sim.view.k * sim.nodes[i].r > 8.0)
            .count();
        assert!(
            readable * 2 > sim.active.len(),
            "only {readable} of {} names are drawn",
            sim.active.len()
        );
    }

    #[test]
    fn a_grabbed_node_stays_where_it_is_put() {
        let graph = vault(5);
        let mut sim = measured(&graph);
        settle(&mut sim);
        sim.nodes[0].fx = Some(500.0);
        sim.nodes[0].fy = Some(-250.0);
        sim.alpha = 0.3;
        for _ in 0..30 {
            sim.tick();
        }
        assert_eq!((sim.nodes[0].x, sim.nodes[0].y), (500.0, -250.0));
    }

    #[test]
    fn search_puts_a_name_match_before_a_path_match() {
        let graph = vault(6);
        let mut sim = measured(&graph);
        settle(&mut sim);
        let found = sim.search("daily");
        assert!(!found.is_empty(), "found nothing");
        assert!(
            sim.nodes[found[0]].label.contains("daily"),
            "{} matched only by path",
            sim.nodes[found[0]].label
        );
        assert!(sim.search("   ").is_empty(), "an empty query finds nothing");
    }

    #[test]
    fn hit_testing_finds_the_node_under_the_pointer() {
        let graph = vault(8);
        let mut sim = measured(&graph);
        settle(&mut sim);
        let target = sim.active[3];
        let (nx, ny) = (sim.nodes[target].x, sim.nodes[target].y);
        let sx = (nx - sim.view.x) * sim.view.k + (sim.viewport.w + sim.viewport.panel) / 2.0;
        let sy = (ny - sim.view.y) * sim.view.k + sim.viewport.h / 2.0;
        assert_eq!(sim.hit(sx, sy), Some(target));
        assert_eq!(sim.hit(sx + 5000.0, sy), None, "empty space hits nothing");
    }
}
