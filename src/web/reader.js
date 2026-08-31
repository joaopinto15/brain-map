// Link targets resolve the way `links.rs` resolves them, so a link that drew an edge
// in the graph is a link the reader can follow: wikilinks by note name or by path,
// markdown links relative to the linking note, or bundle-absolute from the vault root.
const normalize = path => {
  const parts = [];
  for (const part of path.split('/')) {
    if (part === '..') parts.pop();
    else if (part && part !== '.') parts.push(part);
  }
  return parts.join('/');
};
const key = path => normalize(path).replace(/\.md$/i, '').toLowerCase();
const byKey = new Map();
for (const n of nodes) {
  if (n.id.startsWith('__')) continue;
  const stem = n.id.split('/').pop().replace(/\.md$/i, '').toLowerCase();
  if (!byKey.has(stem)) byKey.set(stem, n);
  if (!byKey.has(key(n.id))) byKey.set(key(n.id), n);
}
function resolveLink(target, from, relative) {
  const clean = unesc(target).split('#')[0].trim();
  if (!clean) return null;
  if (relative && !clean.startsWith('/')) {
    const dir = (from || '').split('/').slice(0, -1).join('/');
    return byKey.get(key(dir ? `${dir}/${clean}` : clean));
  }
  return byKey.get(key(clean));
}

// Clicking a note opens its source next to the graph; folder and vault nodes have none.
const readerBody = document.getElementById('body');
let reading = null;
function openNote(n) {
  if (!n || n.id.startsWith('__')) return closeNote();
  reading = n.id;
  hidePanel(false);
  document.body.classList.add('reading');
  markTree(n.id);
  document.getElementById('title').textContent = n.label;
  document.getElementById('path').textContent = n.id;
  document.getElementById('tags').innerHTML = (n.tags || [])
    .map(t => `<span>${tagIcon(t)} ${t}</span>`).join('');
  readerBody.innerHTML = '<p class="empty">Loading…</p>';
  fetch('/note?path=' + encodeURIComponent(n.id))
    .then(r => r.ok ? r.text() : Promise.reject(r.status))
    .then(text => {
      if (reading !== n.id) return;
      readerBody.innerHTML = text.trim() ? markdown(text) : '<p class="empty">This note is empty.</p>';
      readerBody.scrollTop = 0;
    })
    .catch(() => { if (reading === n.id) readerBody.innerHTML = '<p class="empty">Could not read this note.</p>'; });
}
function closeNote() {
  reading = null;
  markTree(null);
  document.body.classList.remove('reading');
  document.body.classList.remove('full');
}
// Reading a long note in a 420px column is the thing the panel is worst at, so it can
// take the whole window instead. Closing the note drops it — a full-width panel with
// nothing in it would just hide the graph.
function toggleFull() {
  if (reading) document.body.classList.toggle('full');
}
readerBody.addEventListener?.('click', e => {
  const a = e.target.closest?.('a[data-note]');
  if (!a) return;
  e.preventDefault();
  const n = resolveLink(a.dataset.note, reading, 'rel' in a.dataset);
  if (n) goTo(n);
  else a.classList.add('missing');
});
document.getElementById('full').onclick = toggleFull;
document.getElementById('close').onclick = () => { selected = null; closeNote(); };
document.getElementById('edit').onclick = () => {
  if (reading) fetch('/edit?path=' + encodeURIComponent(reading));
};

// The vault as a folder tree. <details> carries the open/closed state, so there is
// none to track here.
const tree = document.getElementById('tree');
const treeRoot = { dirs: new Map(), files: [] };
for (const n of nodes) {
  if (n.id.startsWith('__')) continue;
  const parts = n.id.split('/');
  let dir = treeRoot;
  for (const name of parts.slice(0, -1)) {
    if (!dir.dirs.has(name)) dir.dirs.set(name, { dirs: new Map(), files: [] });
    dir = dir.dirs.get(name);
  }
  dir.files.push(n);
}
function renderDir(dir) {
  let html = '';
  for (const [name, sub] of dir.dirs) {
    html += `<details open><summary>📁 <span class="nm" title="${attr(escHtml(name))}">` +
            `${escHtml(name)}</span></summary>` +
            `<div class="kids">${renderDir(sub)}</div></details>`;
  }
  for (const f of dir.files) {
    html += `<div class="file" data-i="${f.i}"><span class="ic">${nodeIcon(f)}</span>` +
            `<span class="nm" title="${attr(escHtml(f.id))}">${escHtml(f.label)}</span></div>`;
  }
  return html;
}
tree.innerHTML = renderDir(treeRoot);
tree.addEventListener?.('click', e => {
  const row = e.target.closest?.('.file');
  if (!row) return;
  goTo(nodes[+row.dataset.i]);
});
function markTree(id) {
  for (const row of tree.querySelectorAll?.('.file') || []) {
    row.classList.toggle('on', nodes[+row.dataset.i].id === id);
  }
}

// The explorer's width is the reader's own business. Pointer capture keeps the drag alive
// over the canvas, and the width outlives the run the way the theme and the growth do.
const grip = document.getElementById('grip');
const PANEL_KEY = 'brain-map-panel';
try { setPanel(+localStorage.getItem(PANEL_KEY) || panelWidth); } catch {}
grip?.addEventListener?.('pointerdown', e => {
  e.preventDefault();
  grip.setPointerCapture(e.pointerId);
  grip.classList.add('on');
});
grip?.addEventListener?.('pointermove', e => {
  if (grip.hasPointerCapture?.(e.pointerId)) setPanel(e.clientX);
});
grip?.addEventListener?.('pointerup', e => {
  grip.releasePointerCapture(e.pointerId);
  grip.classList.remove('on');
  try { localStorage.setItem(PANEL_KEY, panelWidth); } catch {}
});
