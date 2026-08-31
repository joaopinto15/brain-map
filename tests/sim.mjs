// Headless check that the force simulation settles instead of jittering.
// Runs the real page script against a synthetic graph. The script is assembled the
// way src/page.rs assembles it — that file's include_str! list is the only place the
// order lives, so a part the server does not ship is a part this harness does not run.
//   node tests/sim.mjs
import { readFileSync } from 'node:fs';

const read = name => readFileSync(new URL(`../src/${name}`, import.meta.url), 'utf8');
const parts = [...read('page.rs').matchAll(/include_str!\("web\/([\w.]+)"\)/g)].map(m => m[1]);
const script = parts.filter(f => f.endsWith('.js')).map(f => read(`web/${f}`)).join('');
const html = read('web/page.html')
  .replace('__STYLE__', read('web/style.css'))
  .replace('__SCRIPT__', script);

const groups = {
  router: { c: '#34d399', r: 11, glow: 30, name: 'Vault', pace: 0, pause: 0 },
  ideas: { c: '#60a5fa', r: 6, glow: 14, name: 'ideas', pace: 200, pause: 450 },
  daily: { c: '#fbbf24', r: 6, glow: 14, name: 'daily', pace: 200, pause: 450 },
};
const nodes = [{ id: '__vault__', label: 'vault', g: 'router' }];
const links = [];
for (const [g, count] of [['ideas', 40], ['daily', 40]]) {
  const dir = nodes.push({ id: `__dir__${g}`, label: g, g }) - 1;
  links.push({ s: 0, t: dir });
  for (let i = 0; i < count; i++) {
    const tags = i % 5 === 0 ? { tags: [g === 'ideas' ? 'research' : 'journal', 'tag-' + (i % 3)] } : {};
    const n = nodes.push({ id: `${g}/${i}.md`, label: `${g}-${i}`, g, ...tags }) - 1;
    links.push({ s: dir, t: n });
    if (i > 2) links.push({ s: n, t: n - 3 });
  }
}
const icons = { ideas: '\u{1f4a1}', daily: '\u{1f4c5}', research: '\u{1f52c}', 'tag-0': '\u{2b50}' };
const GRAPH = { vault: '/tmp/vault', groups, nodes, links, icons };

const noop = new Proxy(() => noop, { get: () => noop });
// A real class list, not a no-op: `reading` and `full` are page state the reader owns,
// and a stub that swallows them cannot catch one being left behind.
const cssVars = new Map();   // what applyTheme actually pushed, per theme
const classList = () => {
  const on = new Set();
  return { has: c => on.has(c), add: c => on.add(c), remove: c => on.delete(c),
    toggle: (c, force) => (force ?? !on.has(c)) ? on.add(c) : on.delete(c) };
};
const panel = () => ({ innerHTML: '', textContent: '', scrollTop: 0, classList: classList() });
// Elements that record their listeners so the page's own wiring can be exercised. A stub
// that swallows addEventListener cannot tell working wiring from none at all.
const el = (extra = {}) => {
  const handlers = {};
  return {
    addEventListener: (type, fn) => (handlers[type] ??= []).push(fn),
    fire(type, ev = {}) {
      for (const fn of handlers[type] || []) fn({ target: this, preventDefault() {}, stopPropagation() {}, ...ev });
    },
    value: '', checked: false, textContent: '', innerHTML: '', tagName: 'INPUT',
    focused: false, focus() { this.focused = true; }, blur() { this.focused = false; },
    close() { this.open = false; }, showModal() { this.open = true; }, open: false,
    ...extra,
  };
};
const searchInput = el();
searchInput.press = key => searchInput.fire('keydown', { key });
const widthSlider = el(), treeToggle = el(), settingsDialog = el();
const windowKeys = [];
const elements = { count: { textContent: '' }, legend: { innerHTML: '' }, q: searchInput,
  panelw: widthSlider, showtree: treeToggle, settings: settingsDialog,
  panelwv: el(), vaultnow: el(), cfg: el(), cfgclose: el(), sw: el(),
  reader: panel(), body: panel(), title: panel(), path: panel(), tags: panel(), close: {},
  full: {},
  tree: panel() };
const drawn = [];   // fillText calls from the last frame, so icon placement is checkable
const canvas = {
  getContext: () => new Proxy({}, { get: (_, k) => k === 'fillText'
    ? (text, x, y) => drawn.push({ text, x, y })
    // Labels drive how far apart the collide pass holds nodes, so the harness has to
    // answer with a width that grows with the name rather than a stub constant.
    : k === 'measureText' ? text => ({ width: text.length * 6 })
    : () => ({ addColorStop: () => {} }) }),
  addEventListener: () => {}, classList: { add: () => {}, remove: () => {} }, style: {},
};
const document = {
  documentElement: { style: { setProperty: (k, v) => cssVars.set(k, v) } },
  body: { classList: classList() },
  getElementById: id => (id === 'c' ? canvas : elements[id] ?? { addEventListener: () => {} }),
};

const run = new Function(
  '__GRAPH_JSON__', 'document', 'performance', 'requestAnimationFrame',
  'devicePixelRatio', 'innerWidth', 'innerHeight', 'addEventListener', 'fetch', 'setInterval',
  script.replace('__GRAPH__', 'JSON.parse(__GRAPH_JSON__)') +
  '\nreturn { frame, state: () => ({ active, activeLinks, alpha, done }), THEMES, applyTheme, groupColor, tagIcon, nodeIcon, markdown, resolveLink, renderDir, treeRoot, setFilter, matchesFilter, schedule, setPanel, openNote, closeNote, toggleFull, hidePanel, search, step, legendHtml: () => legend.innerHTML,\n  toWorld, panel: () => PANEL, view: () => view, selected: () => selected, matches: () => found, changed, drawSettings };'
);
// Nodes are born at a random angle, so every measurement below would otherwise wander by
// a few pixels between runs. Seeded, the harness reports the same numbers every time and a
// threshold can sit close to the real value instead of well clear of the noise.
let seed = 0x2f6e2b1;
Math.random = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);

const api = run(JSON.stringify(GRAPH), document, { now: () => 0 }, () => {}, 1, 1600, 900,
  (type, fn) => { if (type === 'keydown') windowKeys.push(fn); },
  () => Promise.reject('no server in the harness'), () => 0);

const STEP = 16;
let now = 0, settledAt = null, grownAt = null, peak = 0;
for (let f = 0; f < 6000; f++) {
  const before = api.state().active.map(n => [n.x, n.y]);
  now += STEP;
  api.frame(now);
  const { active, alpha, done } = api.state();
  if (done && grownAt === null) grownAt = now;
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
  cssVars.clear();
  api.applyTheme(key);
  for (const g of Object.keys(GRAPH.groups)) {
    const c = api.groupColor(g);
    if (!/^#[0-9a-f]{6}$/i.test(c)) themeFail.push(`${key}/${g} gave ${c}`);
  }
  // A theme that misses a key in CSS_KEYS writes `undefined` into the page and the
  // chrome or the note renderer silently loses a colour.
  for (const [prop, value] of cssVars) {
    if (!/^--[a-z-]+$/.test(prop)) themeFail.push(`${key} wrote a bad property name ${prop}`);
    if (typeof value !== 'string' || !value) themeFail.push(`${key} left ${prop} as ${value}`);
  }
  // The three the note renderer needs, named so a rename cannot pass unnoticed.
  for (const prop of ['--heading', '--strong', '--code', '--accent', '--border', '--muted']) {
    if (!cssVars.has(prop)) themeFail.push(`${key} never set ${prop}`);
  }
  now += STEP;
  api.frame(now);
}
api.applyTheme('midnight');

const { active } = api.state();
drawn.length = 0;
now += STEP; api.frame(now);
const far = active.filter(n => !Number.isFinite(n.x) || Math.abs(n.x) > 1e4 || Math.abs(n.y) > 1e4);
let worstOverlap = 0;
for (let i = 0; i < active.length; i++) {
  for (let j = i + 1; j < active.length; j++) {
    const a = active[i], b = active[j];
    const gap = a.r + b.r - Math.hypot(a.x - b.x, a.y - b.y);
    worstOverlap = Math.max(worstOverlap, gap);
  }
}
// Names are what the reader actually has to tell apart. A label sits above its node in a
// band one line tall, so two of them collide only when they share that band and their
// widths reach across: that pair is what the squashed collide is there to prevent.
// Measured off the label itself, the way the stub draws it — reading `hw` back would only
// prove the collide agrees with itself.
const labelHw = n => n.label.length * 3;
let worstLabels = 0;
for (let i = 0; i < active.length; i++) {
  for (let j = i + 1; j < active.length; j++) {
    const a = active[i], b = active[j];
    const ay = a.y - a.r, by = b.y - b.r;
    if (Math.abs(ay - by) > 11) continue;
    worstLabels = Math.max(worstLabels, labelHw(a) + labelHw(b) - Math.abs(a.x - b.x));
  }
}
// Sections have to read as sections: a link that leaves its folder must end up visibly
// longer than one that stays inside it, or the whole graph is a single even mesh.
const span = l => Math.hypot(l.source.x - l.target.x, l.source.y - l.target.y);
const mean = ls => ls.reduce((t, l) => t + span(l), 0) / (ls.length || 1);
const inside = mean(api.state().activeLinks.filter(l => !l.far));
const across = mean(api.state().activeLinks.filter(l => l.far));

// Spreading the folders apart costs zoom: the camera fits the whole graph, so a layout
// pushed too far open comes back with every name below the size the renderer will draw.
const fitK = api.view().k;
const readable = active.filter(n => fitK * n.r > 8).length;
let drift = 0;
const before = active.map(n => [n.x, n.y]);
for (let f = 0; f < 30; f++) { now += STEP; api.frame(now); }
active.forEach((n, i) => { drift = Math.max(drift, Math.hypot(n.x - before[i][0], n.y - before[i][1])); });

const fail = [];
if (active.length !== nodes.length) fail.push(`only ${active.length}/${nodes.length} nodes grew in`);
if (settledAt === null) fail.push('simulation never cooled below the alpha floor');
// The schedule is squeezed into one budget however many groups a vault has, so the last
// node is in within it. The fixture overruns on purpose — without the squeeze it needs 17s.
if (grownAt > 9000) fail.push(`the vault took ${(grownAt / 1000).toFixed(1)}s to grow in`);
if (far.length) fail.push(`${far.length} nodes flew off to infinity`);
if (peak > 60) fail.push(`nodes jumped ${peak.toFixed(1)}px in one frame`);
if (worstOverlap > 6) fail.push(`nodes overlap by ${worstOverlap.toFixed(1)}px`);
if (worstLabels > 8) fail.push(`labels overlap by ${worstLabels.toFixed(1)}px`);
if (across < inside * 1.5) fail.push(
  `folders are not separated: ${across.toFixed(0)}px across vs ${inside.toFixed(0)}px inside`);
if (readable * 2 < active.length)
  fail.push(`only ${readable}/${active.length} names are drawn at the settled zoom`);
if (drift > 0.5) fail.push(`still drifting ${drift.toFixed(2)}px/frame after settling`);
if (themeFail.length) fail.push(`bad theme colours: ${themeFail.join(', ')}`);

// Icons come off the wire; an unknown tag resolves to nothing rather than a guess.
const iconFail = [];
if (api.tagIcon('research') !== icons.research) iconFail.push('known tag lost its wire icon');
if (api.tagIcon('a tag nobody has ever written') !== '') iconFail.push('unknown tag invented an icon');
if (iconFail.length) fail.push(`tag icons: ${iconFail.join(', ')}`);

// Every node draws one principal emoji: its first tag, or its group when it has none.
const tagged = active.find(n => n.tags), bare = active.find(n => !n.tags && api.tagIcon(n.g));
const nodeIconFail = [];
if (api.nodeIcon(tagged) !== api.tagIcon(tagged.tags[0])) nodeIconFail.push('tagged node ignores its first tag');
if (api.nodeIcon(bare) !== api.tagIcon(bare.g)) nodeIconFail.push('untagged node does not fall back to its group');
if (nodeIconFail.length) fail.push(`node icons: ${nodeIconFail.join(', ')}`);

// ...and it is painted at the node's centre, inside the disc, not beside the label.
const centred = active.filter(n => drawn.some(d => d.text === api.nodeIcon(n) && d.x === n.x && d.y === n.y));
const centres = new Set(active.map(n => `${n.x},${n.y}`));
const beside = drawn.filter(d => /\p{Extended_Pictographic}/u.test(d.text) && !centres.has(`${d.x},${d.y}`));
if (!centred.length) fail.push('no node drew its icon at its centre');
if (beside.length) fail.push(`${beside.length} emoji drawn outside a node`);

// The reader's markdown: the blocks notes actually use, and no raw HTML slipping through.
const mdChecks = [
  ['# Title', '<h1>Title</h1>'],
  ['- [ ] todo\n- [x] done', '<ul><li>\u2610 todo</li><li>\u2611 done</li></ul>'],
  ['1. one\n2. two', '<ol><li>one</li><li>two</li></ol>'],
  ['> quoted', '<blockquote><p>quoted</p></blockquote>'],
  ['see **bold** and `code`', '<p>see <b>bold</b> and <code>code</code></p>'],
  ['a [[Wiki Link|alias]] here', '<p>a <a class="wiki" data-note="Wiki Link">alias</a> here</p>'],
  ['[next](../ideas/1.md)', '<p><a class="wiki" data-note="../ideas/1.md" data-rel>next</a></p>'],
  ['[docs](https://x.com)', '<p><a href="https://x.com" target="_blank" rel="noopener">docs</a></p>'],
  ['```\nraw <b>x</b>\n```', '<pre><code>raw &lt;b&gt;x&lt;/b&gt;</code></pre>'],
  ['<script>alert(1)</script>', '<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>'],
  ['<!--toc:start-->\ntext', '<p>text</p>'],
  ['---\ntype: Note\ntags: [a]\n---\n\n# Real', '<h1>Real</h1>'],
  // Consecutive lines are one paragraph; a blank line starts the next.
  ['one line\nand its rest', '<p>one line and its rest</p>'],
  ['first para\n\nsecond para', '<p>first para</p><p>second para</p>'],
  // A quote runs until a blank line — a plain line after it is a lazy continuation.
  ['lead in\n> quoted one\n> quoted two\nafter',
   '<p>lead in</p><blockquote><p>quoted one quoted two after</p></blockquote>'],
  ['> quoted\n\nafter', '<blockquote><p>quoted</p></blockquote><p>after</p>'],
  // Tables: header, alignment, ragged rows padded out.
  ['| a | b |\n|---|---|\n| 1 | 2 |',
   '<div class="scroll"><table><thead><tr><th>a</th><th>b</th></tr></thead>' +
   '<tbody><tr><td>1</td><td>2</td></tr></tbody></table></div>'],
  ['| a | b |\n|:--|--:|\n| 1 |',
   '<div class="scroll"><table><thead><tr><th style="text-align:left">a</th>' +
   '<th style="text-align:right">b</th></tr></thead><tbody><tr>' +
   '<td style="text-align:left">1</td><td style="text-align:right"></td></tr></tbody></table></div>'],
  // Nested lists sit inside the item above them.
  ['- a\n  - b\n- c', '<ul><li>a<ul><li>b</li></ul></li><li>c</li></ul>'],
  ['- a\n  1. b\n  2. c', '<ul><li>a<ol><li>b</li><li>c</li></ol></li></ul>'],
  ['- a\n\n- b', '<ul><li>a</li><li>b</li></ul>'],
  ['- a\n  wrapped', '<ul><li>a wrapped</li></ul>'],
  // Inline: code is never rewritten from the inside, plus strikethrough and autolinks.
  ['`a_*b*_c`', '<p><code>a_*b*_c</code></p>'],
  ['~~gone~~', '<p><del>gone</del></p>'],
  ['<https://x.com>', '<p><a href="https://x.com" target="_blank" rel="noopener">https://x.com</a></p>'],
  ['![pic](https://x.com/a.png)', '<p><img src="https://x.com/a.png" alt="pic" loading="lazy"></p>'],
  ['```js\nlet x\n```', '<pre><code class="language-js">let x</code></pre>'],
  // Footnotes: the reference carries the text, the definition lands at the bottom.
  ['cited[^a]\n\n[^a]: the source',
   '<p>cited<sup class="fn" title="the source">1</sup></p><hr><ol class="fn"><li id="fn-a">the source</li></ol>'],
];
const mdFail = mdChecks.filter(([src, want]) => api.markdown(src) !== want)
  .map(([src, want]) => `${JSON.stringify(src)} gave ${JSON.stringify(api.markdown(src))} want ${JSON.stringify(want)}`);
if (mdFail.length) fail.push(`markdown: ${mdFail.join(' | ')}`);

// Following a link from the reader resolves the way links.rs resolved it into an edge.
const target = nodes.find(n => n.id === 'ideas/1.md');
const linkChecks = [
  ['by note name', api.resolveLink('1', null, false), target],
  ['by path', api.resolveLink('ideas/1.md', null, false), target],
  ['by path without the extension', api.resolveLink('ideas/1', null, false), target],
  ['relative to the linking note', api.resolveLink('1.md', 'ideas/0.md', true), target],
  ['bundle-absolute', api.resolveLink('/ideas/1.md', 'daily/0.md', true), target],
  ['up and over', api.resolveLink('../ideas/1.md', 'daily/0.md', true), target],
  ['past an anchor', api.resolveLink('ideas/1.md#section', null, false), target],
  ['a target nobody wrote', api.resolveLink('nowhere.md', 'ideas/0.md', true), undefined],
  ['an empty target', api.resolveLink('', null, false), null],
];
const linkFail = linkChecks.filter(([, got, want]) => (got?.id ?? got) !== (want?.id ?? want))
  .map(([what, got, want]) => `${what}: got ${got?.id ?? got} want ${want?.id ?? want}`);
if (linkFail.length) fail.push(`reader links: ${linkFail.join(' | ')}`);

// The bar's growth picker rebuilds the schedule. Rebuilding is from scratch, so asking
// twice for the same budget must land in the same place rather than compounding.
const lastAt = () => Math.max(...api.state().active.map(n => n.t));
const budgetFail = [];
api.schedule(Infinity);
const natural = lastAt() - 600;   // what this fixture takes when no budget squeezes it
for (const ms of [3000, 30000, 3000, 3000, 0]) {
  api.schedule(ms);
  // A budget looser than the vault needs leaves it at its own pace.
  const want = Math.min(ms, natural);
  if (Math.abs(lastAt() - 600 - want) > 1) budgetFail.push(`${ms}ms budget grew in ${lastAt() - 600}ms`);
}
if (natural < 9000) budgetFail.push(`fixture only takes ${natural}ms — it no longer tests the squeeze`);
if (budgetFail.length) fail.push(`growth budget: ${budgetFail.join(', ')}`);
api.schedule(8000);

// Dragging the grip resizes the explorer. The width is clamped so neither the panel nor
// the canvas can be squeezed away, and the graph's centre has to follow it — a panel that
// moves without the canvas following puts every node under the wrong pointer.
const panelFail = [];
const widthChecks = [[600, 600], [50, 220], [50000, 1600 - 200]];
for (const [asked, want] of widthChecks) {
  api.setPanel(asked);
  if (api.panel() !== want) panelFail.push(`asked ${asked}px, got ${api.panel()}px, want ${want}px`);
}
api.setPanel(300);
const [narrowX] = api.toWorld(800, 450);
api.setPanel(700);
const [wideX] = api.toWorld(800, 450);
if (!(wideX < narrowX)) panelFail.push('the canvas centre did not follow the panel');
if (panelFail.length) fail.push(`explorer width: ${panelFail.join(', ')}`);
api.setPanel(420);

// Reading a note fullscreen is two classes on the body. Both have to come off when the
// note closes: a full-width panel holding nothing would sit over the graph with no way
// back to it.
const reading = api.state().active.find(n => !n.id.startsWith('__'));
const fullFail = [];
const cls = c => document.body.classList.has(c);
api.toggleFull();
if (cls('full')) fullFail.push('went fullscreen with no note open');
api.openNote(reading);
if (!cls('reading')) fullFail.push('opening a note did not enter reading mode');
api.toggleFull();
if (!cls('full')) fullFail.push('F did not go fullscreen');
api.toggleFull();
if (cls('full')) fullFail.push('F did not come back out');
api.toggleFull();
api.closeNote();
if (cls('full') || cls('reading')) fullFail.push('closing the note left the panel over the graph');
if (fullFail.length) fail.push(`fullscreen reading: ${fullFail.join(', ')}`);

// `space e` hides the explorer. The width has to go back to the graph, or the canvas
// keeps centring around a panel that is not on screen.
const hideFail = [];
api.hidePanel(true);
if (api.panel() !== 0) hideFail.push(`hidden explorer still reserves ${api.panel()}px`);
api.hidePanel(false);
if (api.panel() !== 420) hideFail.push(`unhiding gave back ${api.panel()}px, want 420px`);
api.hidePanel(true);
api.openNote(reading);              // a note cannot open into a panel that is display:none
if (api.panel() !== 420) hideFail.push('opening a note left the explorer hidden');
api.closeNote();
if (hideFail.length) fail.push(`hide explorer: ${hideFail.join(', ')}`);

// `/` then n/N walks every match, wrapping, with label matches ahead of path-only ones.
const searchFail = [];
api.search('ideas-1');
const walk = [];
for (let i = 0; i < 13; i++) { walk.push(api.selected()?.label); api.step(1); }
const wanted = ['ideas-1', 'ideas-10', 'ideas-11', 'ideas-12', 'ideas-13', 'ideas-14',
                'ideas-15', 'ideas-16', 'ideas-17', 'ideas-18', 'ideas-19', 'ideas-1', 'ideas-10'];
if (walk.join() !== wanted.join()) searchFail.push(`n walked ${walk.join(' ')}`);
api.search('ideas-1');
api.step(-1);
if (api.selected()?.label !== 'ideas-19') searchFail.push(`N from the first match gave ${api.selected()?.label}`);
// A note whose label and path both match is one hit, not two, and a path-only match
// still counts — the two passes must not overlap or leave anything out.
api.search('daily');
if (new Set(api.matches()).size !== api.matches().length) searchFail.push('a node matched twice');
if (api.matches().some(n => !`${n.label} ${n.id}`.toLowerCase().includes('daily'))) {
  searchFail.push('a node that does not match came back');
}
api.search('.md');
if (api.matches().length !== 80) searchFail.push(`path-only search found ${api.matches().length}, want 80`);
api.search('   ');
if (api.matches().length) searchFail.push('an empty query matched something');
api.search('nothing matches this');
const stuck = api.selected();
api.step(1);
if (api.selected() !== stuck) searchFail.push('n moved with no matches');
if (searchFail.length) fail.push(`search: ${searchFail.join(', ')}`);

// The vault watcher reloads when the fingerprint moves. The first answer is only a
// baseline — treating it as a change would reload the page forever.
const watchChecks = [
  ['the first answer', api.changed(null, '7'), false],
  ['an unchanged vault', api.changed('7', '7'), false],
  ['a changed vault', api.changed('7', '9'), true],
  ['a vault with no notes', api.changed('0', '0'), false],
];
const watchFail = watchChecks.filter(([, got, want]) => got !== want).map(([what]) => what);
if (watchFail.length) fail.push(`vault watch: ${watchFail.join(', ')} read wrong`);

// The keys as a user presses them, through the listeners the page actually registered.
// The functions behind them are checked above; this checks that anything is wired to them.
const press = (key, target) => windowKeys.forEach(fn => fn({ key, target: target ?? { tagName: 'CANVAS' },
  preventDefault() {}, stopPropagation() {} }));
const keyFail = [];
if (!windowKeys.length) keyFail.push('nothing listens for keys at all');
if (!searchInput.focused) {
  press('/');
  if (!searchInput.focused) keyFail.push('/ did not focus the search box');
}
// A leader left armed by a stray space must not eat the next key. Only `e` is its.
searchInput.focused = false;
press(' ');
press('/');
if (!searchInput.focused) keyFail.push('a stray space swallowed /');
press(' ');
press('e');
if (api.panel() !== 0) keyFail.push('space e did not hide the explorer');
press(' ');
press('e');
if (api.panel() !== 420) keyFail.push('space e did not bring it back');
searchInput.value = 'ideas-7';
searchInput.press('Enter');
if (api.selected()?.label !== 'ideas-7') keyFail.push(`Enter searched to ${api.selected()?.label}`);
press('n');
if (api.selected()?.label !== 'ideas-7') keyFail.push('n did not walk from the search');
searchInput.value = 'daily';
searchInput.press('Enter');
const first = api.selected()?.label;
press('n');
if (api.selected()?.label === first) keyFail.push('n did not advance');
press('N');
if (api.selected()?.label !== first) keyFail.push('N did not go back');
// Typing must never trigger a binding: r, f and / are all letters someone will type.
const wasDone = api.state().done;
press('r', { tagName: 'INPUT' });
if (api.state().done !== wasDone) keyFail.push('typing r in a field replayed the growth');
if (keyFail.length) fail.push(`keys: ${keyFail.join(', ')}`);

// The bar holds what you do; the dialog holds what you set. A control that drifts back
// into the bar, or a setting that never made it into the dialog, is the thing to catch.
const bar = html.match(/<div id="bar">[\s\S]*?<\/div>/)[0];
const dialog = html.match(/<dialog id="settings">[\s\S]*?<\/dialog>/)[0];
const cfgFail = [];
if (/<select/.test(bar)) cfgFail.push('a select is still in the bar');
for (const id of ['theme', 'speed', 'panelw', 'showtree', 'sw']) {
  if (!dialog.includes(`id="${id}"`)) cfgFail.push(`${id} is not in the settings dialog`);
  if (bar.includes(`id="${id}"`)) cfgFail.push(`${id} is still in the bar`);
}
if (!bar.includes('id="cfg"')) cfgFail.push('no settings button in the bar');

// Both ways of setting the explorer width write the same state, and the dialog shows it.
api.setPanel(420);
widthSlider.value = '600';
widthSlider.fire('input');
if (api.panel() !== 600) cfgFail.push(`the width slider set ${api.panel()}px, want 600px`);
api.setPanel(340);
api.drawSettings();
if (widthSlider.value !== 340) cfgFail.push(`dragging the grip left the slider at ${widthSlider.value}`);

// The checkbox is the same switch as `space e`, read and written from the same place.
treeToggle.checked = false;
treeToggle.fire('change');
if (api.panel() !== 0) cfgFail.push('unchecking Show explorer did not hide it');
api.drawSettings();
if (treeToggle.checked !== false) cfgFail.push('the dialog disagreed with the hidden explorer');
press(' ');
press('e');
api.drawSettings();
if (treeToggle.checked !== true) cfgFail.push('space e did not move the checkbox back');
if (cfgFail.length) fail.push(`settings: ${cfgFail.join(', ')}`);
api.setPanel(420);
api.closeNote();

// Legend rows filter the graph: a group holds every note in it, a tag only the notes
// that declare it, and clicking the same row again clears it.
const lit = () => nodes.filter(api.matchesFilter).length;
api.setFilter('g', 'ideas');
const filterChecks = [
  ['a group lights its notes and the folder node holding them', lit(), 41],
  ['the legend marks the active row', /data-g="ideas"[^>]*/.test(api.legendHtml())
    && / class="row on" data-g="ideas"/.test(api.legendHtml()), true],
];
api.setFilter('tag', 'research');
filterChecks.push(['a tag lights only the notes carrying it', lit(), 8]);
api.setFilter('tag', 'research');
filterChecks.push(['clicking the same row again clears it', lit(), nodes.length]);
api.setFilter('tag', 'tag-0');
filterChecks.push(['a tag shared across groups crosses them', lit(), 6]);
api.setFilter(null);
filterChecks.push(['no filter lights everything', lit(), nodes.length]);
const filterFail = filterChecks.filter(([, got, want]) => got !== want)
  .map(([what, got, want]) => `${what}: got ${got} want ${want}`);
if (filterFail.length) fail.push(`legend filter: ${filterFail.join(' | ')}`);

// The picker hides with the hidden attribute, which an id selector out-specifies unless
// the stylesheet says so again. Without this the card sits on top of the graph forever.
const styled = html.slice(0, html.indexOf('</style>'));
if (/#picker\s*\{[^}]*display:/.test(styled) && !/#picker\[hidden\]\s*\{[^}]*display:\s*none/.test(styled)) {
  fail.push('#picker sets display but never restores it for [hidden]');
}

// Two elements sharing an id means getElementById silently hands back the wrong one.
const ids = [...html.matchAll(/\sid="([^"]+)"/g)].map(m => m[1]);
const duplicated = ids.filter((id, i) => ids.indexOf(id) !== i);
if (duplicated.length) fail.push(`duplicate element ids: ${[...new Set(duplicated)].join(', ')}`);

// The file tree mirrors the vault: a <details> per folder, a clickable row per note.
const treeHtml = api.renderDir(api.treeRoot);
const treeFail = [];
const rows = (treeHtml.match(/class="file"/g) || []).length;
const folders = (treeHtml.match(/<details/g) || []).length;
if (rows !== nodes.filter(n => !n.id.startsWith('__')).length) treeFail.push(`${rows} rows for ${nodes.length} nodes`);
if (folders !== 2) treeFail.push(`${folders} folders, expected ideas + daily`);
if (!/<summary>\u{1f4c1} <span class="nm" title="ideas">ideas<\/span><\/summary>/u.test(treeHtml)) {
  treeFail.push('folder row missing');
}
// Every name sits in its own `.nm` box. The rows are flex, so a bare text node cannot be
// truncated — it would push the tree sideways instead of ending in an ellipsis.
const names = (treeHtml.match(/class="nm"/g) || []).length;
if (names !== rows + folders) treeFail.push(`${names} names boxed, expected ${rows + folders}`);
if (/<\/span>[^<]*[A-Za-z0-9][^<]*<\/div>/.test(treeHtml)) treeFail.push('a name is loose in a row');
// The tooltip carries what the ellipsis cuts, so a truncated name is still readable.
if (!treeHtml.includes('title="ideas/0.md"')) treeFail.push('file row has no full-name tooltip');
if (treeFail.length) fail.push(`tree: ${treeFail.join(', ')}`);

// The explorer is permanent, so the world centre sits in the window to the right of it.
// Miss one of the five origin sites and drawing and hit-testing disagree.
const P = api.panel(), v = api.view();
const [cx, cy] = api.toWorld((1600 + P) / 2, 900 / 2);
const originFail = [];
if (P !== 420) originFail.push(`panel width ${P}, expected 420`);
if (Math.abs(cx - v.x) > 1e-9 || Math.abs(cy - v.y) > 1e-9)
  originFail.push(`centre of the visible area maps to ${cx.toFixed(2)},${cy.toFixed(2)} not ${v.x.toFixed(2)},${v.y.toFixed(2)}`);
if (originFail.length) fail.push(`camera: ${originFail.join(', ')}`);

console.log(`grew in ${(grownAt / 1000).toFixed(1)}s · nodes ${active.length} · settled after ${settledAt} frames · peak step ${peak.toFixed(1)}px ` +
            `· overlap ${worstOverlap.toFixed(1)}px · labels ${worstLabels.toFixed(1)}px ` +
            `· folders ${across.toFixed(0)}/${inside.toFixed(0)}px · fit ${fitK.toFixed(2)} names ${readable}/${active.length} · drift ${drift.toFixed(3)}px ` +
            `· wire icons ${api.tagIcon('research')}${api.tagIcon('tag-0')} · ${centred.length} in nodes ` +
            `· themes ${Object.keys(api.THEMES).join(', ')}`);
if (fail.length) { console.error('FAIL: ' + fail.join('; ')); process.exit(1); }
console.log('ok: graph settles and stays put');
