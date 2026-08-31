const canvas = document.getElementById('c'), ctx = canvas.getContext('2d');
const DPR = Math.min(devicePixelRatio || 1, 2);
let W, H, PANEL, panelWidth = 420, panelHidden = false;
// The explorer's width, in one place: the stylesheet lays out against `--panel-w` and the
// canvas centres against `PANEL`. The grip in the reader calls this; so does every resize,
// since a window that shrank may no longer have room for the width the user chose.
function setPanel(px) {
  panelWidth = Math.max(220, Math.min(px, innerWidth - 200));
  document.documentElement.style.setProperty('--panel-w', panelWidth + 'px');
  // Below 700px the panel covers almost everything and simply overlays instead.
  PANEL = innerWidth < 700 || panelHidden ? 0 : panelWidth;
}
// Hiding the explorer gives its width back to the graph, so it goes through the same
// writer rather than being a CSS-only trick the canvas would never hear about.
function hidePanel(on) {
  panelHidden = on;
  document.body.classList.toggle('hidden-panel', on);
  setPanel(panelWidth);
}
function resize() {
  W = innerWidth; H = innerHeight;
  // The explorer never goes away, so the graph's centre is the middle of what is left.
  setPanel(panelWidth);
  canvas.width = W * DPR; canvas.height = H * DPR;
  canvas.style.width = W + 'px'; canvas.style.height = H + 'px';
}
resize(); addEventListener('resize', resize);

const nodes = GRAPH.nodes.map((n, i) => ({ ...n, i, x: 0, y: 0, vx: 0, vy: 0, fx: null, fy: null }));
const degree = new Array(nodes.length).fill(0);
for (const l of GRAPH.links) { degree[l.s]++; degree[l.t]++; }
const links = GRAPH.links.map(l => {
  const source = nodes[l.s], target = nodes[l.t];
  const count = degree[l.s] + degree[l.t];
  // A link out of its own folder is what separates one section from the next, so it is
  // marked here and rested longer in `sim.js` rather than being told apart mid-tick.
  return { source, target, bias: count ? degree[l.s] / count : 0.5,
    far: source.g !== target.g };
});
for (const n of nodes) {
  const st = STYLE[n.g];
  n.r = Math.min(st.r * 2.2, st.r * (0.8 + Math.sqrt(degree[n.i]) * 0.15));
  n.charge = -(15 + n.r * 6);
}
// What crowds the canvas is the names, not the dots: a label is drawn above its node at a
// fixed screen size and is several times wider than the disc under it. Measuring each once
// gives the collide pass a real horizontal extent to hold apart, so two neighbours may sit
// close without their names running together. `render.js` draws with the same two values.
const LABEL_PX = 11, LABEL_FONT = '-apple-system, system-ui, sans-serif';
ctx.font = `500 ${LABEL_PX}px ${LABEL_FONT}`;
// Capped, so one very long filename cannot open a crater the whole layout has to work around.
const LABEL_HW_MAX = 70;
for (const n of nodes) {
  n.hw = Math.min(Math.max(n.r, ctx.measureText(n.label).width / 2), LABEL_HW_MAX);
}
const neighbours = nodes.map(() => []);
for (const l of links) {
  neighbours[l.source.i].push(l.target.i);
  neighbours[l.target.i].push(l.source.i);
}

// Growth schedule: each group enters as a burst, paced by its own size.
const GROWTH_START = 600;   // a beat before the first node, so the canvas is not born mid-frame
const GROWTH_MS = 8000;     // how long the whole vault may take to assemble, by default

// Left alone, the schedule runs longer the more groups a vault has — a folder costs its
// pause whether it holds three notes or three hundred. Squeezing the finished schedule
// into one budget keeps the relative pacing and bounds the wait; a vault already inside
// the budget is left at its own speed. Rebuildable, since the budget is a picker in the
// bar: call it again and replay.
function schedule(budgetMs) {
  let clockAt = GROWTH_START, prevGroup = null;
  for (const n of nodes) {
    const st = STYLE[n.g];
    if (n.g !== prevGroup) { clockAt += st.pause ?? 500; prevGroup = n.g; }
    clockAt += st.pace ?? 60;
    n.t = clockAt;
  }
  const span = clockAt - GROWTH_START;
  if (span > budgetMs) {
    const squeeze = budgetMs / span;
    for (const n of nodes) n.t = GROWTH_START + (n.t - GROWTH_START) * squeeze;
  }
}
schedule(GROWTH_MS);

