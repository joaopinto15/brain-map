const LINK_DISTANCE = 30, LINK_STRENGTH = 0.5, CHARGE_MAX2 = 450 * 450;
// A link that leaves its folder rests this much longer, so the sections drift into
// islands with visible air between them instead of one even mesh.
const GROUP_GAP = 2.2;
// Labels are wide and short. Counting vertical distance this much heavier turns the
// collide circle into a lozenge the shape of a name, so nodes may stack closely above
// one another while never sitting side by side with their labels overlapping.
const LABEL_SQUASH = 2.5;
const CENTER_STRENGTH = 0.05, GRAVITY = 0.03, COLLIDE_PAD = 3.5, COLLIDE_STRENGTH = 0.9;
const VELOCITY_DECAY = 0.6, ALPHA_DECAY = 0.012, ALPHA_FLOOR = 0.004;

let active = [], activeLinks = [], nextIdx = 0, born = new Set();
let alpha = 0, clock = 0, last = performance.now(), done = false;
let view = { x: 0, y: 0, k: 1.5 }, userCam = false;
let selected = null, hover = null;

// ponytail: O(n²) pair loop, smooth to a few thousand notes; Barnes-Hut past that.
function tick() {
  alpha -= alpha * ALPHA_DECAY;

  for (const l of activeLinks) {
    const s = l.source, t = l.target;
    let dx = t.x + t.vx - s.x - s.vx, dy = t.y + t.vy - s.y - s.vy;
    const d = Math.hypot(dx, dy) || 1e-6;
    const rest = l.far ? LINK_DISTANCE * GROUP_GAP : LINK_DISTANCE;
    const w = (d - rest) / d * alpha * LINK_STRENGTH;
    dx *= w; dy *= w;
    t.vx -= dx * l.bias; t.vy -= dy * l.bias;
    s.vx += dx * (1 - l.bias); s.vy += dy * (1 - l.bias);
  }

  for (let i = 0; i < active.length; i++) {
    const a = active[i];
    for (let j = i + 1; j < active.length; j++) {
      const b = active[j];
      const dx = b.x - a.x, dy = b.y - a.y;
      // Coincident nodes would divide by zero and fling each other off-screen.
      const d2 = Math.max(dx * dx + dy * dy, 1);
      if (d2 < CHARGE_MAX2) {
        a.vx += dx * b.charge * alpha / d2; a.vy += dy * b.charge * alpha / d2;
        b.vx -= dx * a.charge * alpha / d2; b.vy -= dy * a.charge * alpha / d2;
      }
      // `hw` is the node's own half-width including its label, so this keeps names
      // apart, not just discs. The plain distance is only the cheap prefilter — it can
      // never be smaller than the squashed one, so nothing that touches is skipped.
      const touch = a.hw + b.hw + COLLIDE_PAD * 2;
      if (d2 < touch * touch) {
        let cx = b.x + b.vx - a.x - a.vx, cy = (b.y + b.vy - a.y - a.vy) * LABEL_SQUASH;
        const d = Math.hypot(cx, cy) || 1e-6;
        if (d < touch) {
          const push = (touch - d) / d * COLLIDE_STRENGTH;
          cx *= push; cy *= push / LABEL_SQUASH;
          const share = (b.r * b.r) / (a.r * a.r + b.r * b.r);
          b.vx += cx * share; b.vy += cy * share;
          a.vx -= cx * (1 - share); a.vy -= cy * (1 - share);
        }
      }
    }
    a.vx -= a.x * GRAVITY * alpha; a.vy -= a.y * GRAVITY * alpha;
  }

  let cx = 0, cy = 0;
  for (const n of active) { cx += n.x; cy += n.y; }
  cx = cx / active.length * CENTER_STRENGTH; cy = cy / active.length * CENTER_STRENGTH;

  for (const n of active) {
    if (n.fx === null) { n.vx *= VELOCITY_DECAY; n.x += n.vx - cx; }
    else { n.x = n.fx; n.vx = 0; }
    if (n.fy === null) { n.vy *= VELOCITY_DECAY; n.y += n.vy - cy; }
    else { n.y = n.fy; n.vy = 0; }
  }
}

function activate(time) {
  let changed = false;
  while (nextIdx < nodes.length && nodes[nextIdx].t <= time) {
    const n = nodes[nextIdx];
    const anchor = neighbours[n.i].map(j => nodes[j]).find(m => born.has(m.i));
    const angle = Math.random() * 7;
    // Born clear of what it lands next to. Inside the collide radius it would be flung
    // out on its first tick, which is the jolt you see rather than a node growing in.
    const away = anchor ? anchor.hw + n.hw + COLLIDE_PAD : 30;
    n.x = (anchor ? anchor.x : 0) + Math.cos(angle) * away;
    n.y = (anchor ? anchor.y : 0) + Math.sin(angle) * away;
    n.vx = n.vy = 0;
    active.push(n); born.add(n.i); nextIdx++; changed = true;
  }
  if (changed) {
    activeLinks = links.filter(l => born.has(l.source.i) && born.has(l.target.i));
    alpha = Math.max(alpha, Math.min(0.5, 0.2 + active.length / 800));
  }
}

const toWorld = (sx, sy) => [(sx - (W + PANEL) / 2) / view.k + view.x, (sy - H / 2) / view.k + view.y];
function hit(sx, sy) {
  const [wx, wy] = toWorld(sx, sy);
  for (let i = active.length - 1; i >= 0; i--) {
    const n = active[i], r = Math.max(n.r, 5 / view.k) + 3 / view.k;
    if ((n.x - wx) ** 2 + (n.y - wy) ** 2 <= r * r) return n;
  }
  return null;
}

