function restart() {
  for (const n of nodes) { n.vx = n.vy = 0; n.fx = n.fy = null; }
  active = []; activeLinks = []; nextIdx = 0; born = new Set();
  clock = 0; alpha = 0; done = false; selected = null; hover = null; userCam = false;
  setFilter(null);
  view = { x: 0, y: 0, k: 1.5 };
  closeNote();
}
// The server rescans the vault on every request, so a plain reload is the rescan.
document.getElementById('rl').onclick = () => location.reload();

// Choosing the vault. The server holds no vault until this page names one, and only
// this page knows the token it will accept.
const vaultPicker = document.getElementById('picker');
const pickErr = document.getElementById('pickerr');
const RECENT = 'brain-map-recent';
const remembered = () => {
  try { return JSON.parse(localStorage.getItem(RECENT)) || []; } catch { return []; }
};
function showPicker() {
  const rows = remembered();
  const list = document.getElementById('recent');
  if (list) {
    list.innerHTML = rows.length
      ? '<b>Recent</b>' + rows.map(p => `<div>${escHtml(p)}</div>`).join('')
      : '';
    list.onclick = e => { if (e.target.matches?.('div')) openVault(e.target.textContent); };
  }
  if (vaultPicker) vaultPicker.hidden = false;
  document.getElementById('vaultpath')?.focus?.();
}
// A token belongs to one run of the server. A page left open across a restart carries
// a stale one, so rather than dead-ending on 403 it reloads itself and picks up the new.
async function ask(url) {
  const answer = await fetch(url);
  const said = (await answer.text()).trim();
  if (answer.status === 403) {
    pickErr.textContent = 'this page is older than the server — reloading…';
    setTimeout(() => location.reload(), 600);
    return null;
  }
  if (!answer.ok) { pickErr.textContent = said; return null; }
  return said;
}
async function openVault(path) {
  pickErr.textContent = '';
  const said = await ask(`/open?path=${encodeURIComponent(path)}&t=${TOKEN}`);
  if (said === null) return;
  try {
    localStorage.setItem(RECENT,
      JSON.stringify([path, ...remembered().filter(p => p !== path)].slice(0, 6)));
  } catch {}
  location.reload();
}
// The browser cannot name a folder on disk, so the server opens the desktop's own dialog.
document.getElementById('browse')?.addEventListener?.('click', async () => {
  pickErr.textContent = '';
  const chosen = await ask(`/browse?t=${TOKEN}`);
  if (chosen) { document.getElementById('vaultpath').value = chosen; openVault(chosen); }
  else if (chosen === '') pickErr.textContent = 'no folder chosen';
});
document.getElementById('pick')?.addEventListener?.('submit', e => {
  e.preventDefault();
  openVault(document.getElementById('vaultpath').value);
});
document.getElementById('sw').onclick = () => { settings.close?.(); showPicker(); };
addEventListener('keydown', e => { if (e.key === 'Escape' && vaultPicker && !vaultPicker.hidden) vaultPicker.hidden = true; });
// No vault yet: the page opens on the picker instead of an empty canvas. A vault that
// happens to hold no nodes is still a vault, so it is the path that decides, not the count.
if (!GRAPH.vault) showPicker();

// Clicking a legend row keeps only what it names lit, using the same dimming the
// focus does — a group holds every note in it, a tag only the notes that declare it.
let filter = null;
const matchesFilter = n => !filter
  || (filter.kind === 'g' ? n.g === filter.key : (n.tags || []).includes(filter.key));
function setFilter(kind, key) {
  const same = filter && filter.kind === kind && filter.key === key;
  filter = same || !kind ? null : { kind, key };
  drawLegend();
}

const legend = document.getElementById('legend');
const counts = {}, tagCounts = {};
for (const n of nodes) {
  counts[n.g] = (counts[n.g] || 0) + 1;
  for (const tag of n.tags || []) tagCounts[tag] = (tagCounts[tag] || 0) + 1;
}
const topTags = Object.entries(tagCounts).sort((a, b) => b[1] - a[1]).slice(0, 12);
function drawLegend() {
  const on = (kind, key) => filter && filter.kind === kind && filter.key === key ? ' on' : '';
  const key = s => attr(escHtml(s));
  const groupRows = Object.entries(STYLE)
    .filter(([g]) => counts[g])
    .map(([g, st]) => `<div class="row${on('g', g)}" data-g="${key(g)}">` +
      `<span class="dot" style="background:${groupColor(g)}"></span>${escHtml(st.name)} · ${counts[g]}</div>`)
    .join('');
  const tagRows = topTags
    .map(([tag, n]) => `<div class="row${on('tag', tag)}" data-tag="${key(tag)}">` +
      `<span class="icon">${tagIcon(tag)}</span>${escHtml(tag)} · ${n}</div>`)
    .join('');
  legend.innerHTML = groupRows
    + (tagRows ? `<div class="tags">${tagRows}</div>` : '')
    + `<div class="hint">${filter ? 'click again or Esc to clear' : 'click a row to filter'}</div>`;
}
legend.addEventListener?.('click', e => {
  const row = e.target.closest?.('.row');
  if (!row) return;
  const kind = 'g' in row.dataset ? 'g' : 'tag';
  setFilter(kind, row.dataset[kind]);
});

const picker = document.getElementById('theme');
picker.innerHTML = Object.entries(THEMES)
  .map(([key, t]) => `<option value="${key}">${t.name}</option>`).join('');
let saved = 'midnight';
try { saved = localStorage.getItem('brain-map-theme') || saved; } catch {}
picker.value = THEMES[saved] ? saved : 'midnight';
picker.addEventListener('change', () => applyTheme(picker.value));
applyTheme(picker.value);

// A reload the watcher asked for, not the user: the note that was open comes back and
// the growth is skipped for this load only, leaving the picker's own setting alone.
const RESUME = 'brain-map-resume';
let resume = null;
try {
  resume = sessionStorage.getItem(RESUME);
  sessionStorage.removeItem(RESUME);
} catch {}

// How long the whole vault may take to assemble. Changing it rebuilds the schedule and
// replays, since a budget you cannot see applied is a budget you cannot judge.
const speed = document.getElementById('speed');
const BUDGETS = [['0', 'Instant'], ['3000', '3s'], ['5000', '5s'],
                 [String(GROWTH_MS), '8s'], ['15000', '15s'], ['30000', '30s']];
speed.innerHTML = BUDGETS.map(([ms, name]) => `<option value="${ms}">${name}</option>`).join('');
let budget = String(GROWTH_MS);
try { budget = localStorage.getItem('brain-map-growth') || budget; } catch {}
speed.value = BUDGETS.some(([ms]) => ms === budget) ? budget : String(GROWTH_MS);
speed.addEventListener('change', () => {
  try { localStorage.setItem('brain-map-growth', speed.value); } catch {}
  schedule(+speed.value);
  restart();
});
schedule(resume === null ? +speed.value : 0);

// The bar keeps the things you do; everything you set lives in here. <dialog> handles
// the backdrop and Esc, so this only has to open it and keep the controls in step with
// the state they read from — the width can also be changed by dragging the grip.
const settings = document.getElementById('settings');
const width = document.getElementById('panelw');
const widthValue = document.getElementById('panelwv');
const showTree = document.getElementById('showtree');
function drawSettings() {
  document.getElementById('vaultnow').textContent = GRAPH.vault || 'none chosen';
  width.value = panelWidth;
  widthValue.textContent = panelWidth + 'px';
  showTree.checked = !panelHidden;
}
document.getElementById('cfg').onclick = () => { drawSettings(); settings.showModal?.(); };
document.getElementById('cfgclose').onclick = () => settings.close?.();
width.addEventListener?.('input', () => {
  setPanel(+width.value);
  widthValue.textContent = panelWidth + 'px';
  try { localStorage.setItem(PANEL_KEY, panelWidth); } catch {}
});
showTree.addEventListener?.('change', () => hidePanel(!showTree.checked));
drawSettings();

restart();
if (resume) openNote(nodes.find(n => n.id === resume));
requestAnimationFrame(frame);

// The vault is on disk and can change under the page — a note saved in $EDITOR, a file
// dropped in by something else. There is no notifier to subscribe to, so the page asks
// for the vault's fingerprint and reloads when it moves. The reload is the rescan; the
// server already rebuilds the graph on every page load.
// The first answer is only a baseline. Reloading on it would reload forever.
function changed(mark, now) { return mark !== null && now !== mark; }
let mark = null;
setInterval(async () => {
  try {
    const now = (await (await fetch('/changed')).text()).trim();
    const moved = changed(mark, now);
    mark = now;
    if (!moved) return;
    // This reload was the disk's idea, not the user's: come back to the note that was
    // open, and do not make them sit through the growth animation again.
    try { sessionStorage.setItem(RESUME, reading || ''); } catch {}
    location.reload();
  } catch {}
}, 2000);
