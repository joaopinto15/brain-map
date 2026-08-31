let panning = false, dragging = null, mx = 0, my = 0, moved = 0;
canvas.addEventListener('wheel', e => {
  e.preventDefault();
  userCam = true;
  const k = Math.max(0.1, Math.min(8, view.k * Math.exp(-e.deltaY * 0.0015)));
  const [wx, wy] = toWorld(e.clientX, e.clientY);
  view.x = wx - (e.clientX - (W + PANEL) / 2) / k;
  view.y = wy - (e.clientY - H / 2) / k;
  view.k = k;
}, { passive: false });
canvas.addEventListener('mousedown', e => {
  moved = 0; mx = e.clientX; my = e.clientY;
  dragging = hit(e.clientX, e.clientY);
  if (dragging) {
    dragging.fx = dragging.x; dragging.fy = dragging.y;
    alpha = Math.max(alpha, 0.2);
  } else { panning = true; userCam = true; }
  canvas.classList.add('drag');
});
addEventListener('mousemove', e => {
  moved += Math.abs(e.clientX - mx) + Math.abs(e.clientY - my);
  if (dragging) {
    const [wx, wy] = toWorld(e.clientX, e.clientY);
    dragging.fx = wx; dragging.fy = wy;
    alpha = Math.max(alpha, 0.15);
  } else if (panning) {
    view.x -= (e.clientX - mx) / view.k;
    view.y -= (e.clientY - my) / view.k;
  } else hover = hit(e.clientX, e.clientY);
  mx = e.clientX; my = e.clientY;
});
addEventListener('mouseup', e => {
  if (moved < 5) {
    const n = hit(e.clientX, e.clientY);
    selected = n === selected ? null : n;
    openNote(selected);
  }
  if (dragging) { dragging.fx = null; dragging.fy = null; }
  dragging = null; panning = false;
  canvas.classList.remove('drag');
});
// Searching the way `/` does: one query, every match, and n/N to walk them. A label
// match beats a path-only match, which is the order the search always preferred.
let found = [], foundAt = -1;
function search(q) {
  const needle = q.trim().toLowerCase();
  found = [];
  foundAt = -1;
  if (!needle) return;
  const byLabel = new Set();
  for (const n of active) if (n.label.toLowerCase().includes(needle)) { found.push(n); byLabel.add(n); }
  for (const n of active) if (!byLabel.has(n) && n.id.toLowerCase().includes(needle)) found.push(n);
  step(1);
}
function step(d) {
  if (!found.length) return;
  foundAt = (foundAt + d + found.length) % found.length;
  goTo(found[foundAt]);
}

const searchBox = document.getElementById('q');
const PAN = 60;
// Space is the leader, cleared by whatever key follows it. Only a key the leader binds
// is consumed — swallowing the rest would mean one stray space silently killed the next
// keystroke, which is worse than the chord is worth. `space e` toggles the explorer the
// way it does in a vim file tree.
let leader = false;
addEventListener('keydown', e => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
  if (e.key === ' ') { e.preventDefault(); leader = true; return; }
  const lead = leader;
  leader = false;
  if (lead && e.key === 'e') { e.preventDefault(); return hidePanel(!panelHidden); }
  if (e.key === '/') { e.preventDefault(); return searchBox.focus(); }
  if (e.key === 'Escape') { selected = null; setFilter(null); closeNote(); }
  if (e.key === 'r' || e.key === 'R') restart();
  if (e.key === 'f' || e.key === 'F') toggleFull();
  if (e.key === 'n') step(1);
  if (e.key === 'N') step(-1);
  // hjkl pans by a fixed screen distance, so it moves the same amount at any zoom.
  const pan = { h: [-1, 0], l: [1, 0], k: [0, -1], j: [0, 1] }[e.key];
  if (pan) {
    userCam = true;
    view.x += pan[0] * PAN / view.k;
    view.y += pan[1] * PAN / view.k;
  }
});
searchBox.addEventListener('keydown', e => {
  e.stopPropagation();
  if (e.key === 'Escape') return searchBox.blur();
  if (e.key === 'Enter') search(searchBox.value);
});
function goTo(n) {
  selected = n; userCam = true;
  view.x = n.x; view.y = n.y; view.k = Math.max(view.k, 2.2);
  openNote(n);
}

