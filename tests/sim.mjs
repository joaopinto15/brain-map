// Headless check that the force simulation settles instead of jittering.
// Runs the real page script from src/index.html against a synthetic graph.
//   node tests/sim.mjs
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('../src/index.html', import.meta.url), 'utf8');
const script = html.slice(html.indexOf('<script>') + 8, html.lastIndexOf('</script>'));

const groups = {
  router: { c: '#34d399', r: 11, glow: 30, name: 'Vault', pace: 0, pause: 0 },
  ideas: { c: '#60a5fa', r: 6, glow: 14, name: 'ideas', pace: 40, pause: 450 },
  daily: { c: '#fbbf24', r: 6, glow: 14, name: 'daily', pace: 40, pause: 450 },
};
const nodes = [{ id: '__vault__', label: 'vault', g: 'router' }];
const links = [];
for (const [g, count] of [['ideas', 40], ['daily', 40]]) {
  const dir = nodes.push({ id: `__dir__${g}`, label: g, g }) - 1;
  links.push({ s: 0, t: dir });
  for (let i = 0; i < count; i++) {
    const n = nodes.push({ id: `${g}/${i}.md`, label: `${g}-${i}`, g }) - 1;
    links.push({ s: dir, t: n });
    if (i > 2) links.push({ s: n, t: n - 3 });
  }
}
const GRAPH = { vault: '/tmp/vault', groups, nodes, links };

const noop = new Proxy(() => noop, { get: () => noop });
const elements = { count: { textContent: '' }, legend: { innerHTML: '' }, q: { addEventListener: () => {} }, re: {} };
const canvas = {
  getContext: () => new Proxy({}, { get: () => () => ({ addColorStop: () => {} }) }),
  addEventListener: () => {}, classList: { add: () => {}, remove: () => {} }, style: {},
};
const document = {
  documentElement: { style: { setProperty: () => {} } },
  getElementById: id => (id === 'c' ? canvas : elements[id] ?? { addEventListener: () => {} }),
};

const run = new Function(
  '__GRAPH_JSON__', 'document', 'performance', 'requestAnimationFrame',
  'devicePixelRatio', 'innerWidth', 'innerHeight', 'addEventListener',
  script.replace('__GRAPH__', 'JSON.parse(__GRAPH_JSON__)') +
  '\nreturn { frame, state: () => ({ active, alpha, done }), THEMES, applyTheme, groupColor };'
);
const api = run(JSON.stringify(GRAPH), document, { now: () => 0 }, () => {}, 1, 1600, 900, () => {});

const STEP = 16;
let now = 0, settledAt = null, peak = 0;
for (let f = 0; f < 6000; f++) {
  const before = api.state().active.map(n => [n.x, n.y]);
  now += STEP;
  api.frame(now);
  const { active, alpha, done } = api.state();
  let moved = 0;
  for (let i = 0; i < before.length; i++) {
    moved = Math.max(moved, Math.hypot(active[i].x - before[i][0], active[i].y - before[i][1]));
  }
  if (done) peak = Math.max(peak, moved);
  if (done && alpha <= 0.004 && settledAt === null) settledAt = f;
  if (settledAt !== null && f > settledAt + 120) break;
}

// Every theme must give every group a colour, and switching mid-run must not throw.
const themeFail = [];
for (const key of Object.keys(api.THEMES)) {
  api.applyTheme(key);
  for (const g of Object.keys(GRAPH.groups)) {
    const c = api.groupColor(g);
    if (!/^#[0-9a-f]{6}$/i.test(c)) themeFail.push(`${key}/${g} gave ${c}`);
  }
  now += STEP;
  api.frame(now);
}
api.applyTheme('midnight');

const { active } = api.state();
const far = active.filter(n => !Number.isFinite(n.x) || Math.abs(n.x) > 1e4 || Math.abs(n.y) > 1e4);
let worstOverlap = 0;
for (let i = 0; i < active.length; i++) {
  for (let j = i + 1; j < active.length; j++) {
    const a = active[i], b = active[j];
    const gap = a.r + b.r - Math.hypot(a.x - b.x, a.y - b.y);
    worstOverlap = Math.max(worstOverlap, gap);
  }
}
let drift = 0;
const before = active.map(n => [n.x, n.y]);
for (let f = 0; f < 30; f++) { now += STEP; api.frame(now); }
active.forEach((n, i) => { drift = Math.max(drift, Math.hypot(n.x - before[i][0], n.y - before[i][1])); });

const fail = [];
if (active.length !== nodes.length) fail.push(`only ${active.length}/${nodes.length} nodes grew in`);
if (settledAt === null) fail.push('simulation never cooled below the alpha floor');
if (far.length) fail.push(`${far.length} nodes flew off to infinity`);
if (peak > 60) fail.push(`nodes jumped ${peak.toFixed(1)}px in one frame`);
if (worstOverlap > 6) fail.push(`nodes overlap by ${worstOverlap.toFixed(1)}px`);
if (drift > 0.5) fail.push(`still drifting ${drift.toFixed(2)}px/frame after settling`);
if (themeFail.length) fail.push(`bad theme colours: ${themeFail.join(', ')}`);

console.log(`nodes ${active.length} · settled after ${settledAt} frames · peak step ${peak.toFixed(1)}px ` +
            `· overlap ${worstOverlap.toFixed(1)}px · drift ${drift.toFixed(3)}px ` +
            `· themes ${Object.keys(api.THEMES).join(', ')}`);
if (fail.length) { console.error('FAIL: ' + fail.join('; ')); process.exit(1); }
console.log('ok: graph settles and stays put');
