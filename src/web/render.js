function draw() {
  ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
  ctx.fillStyle = theme.canvasBg; ctx.fillRect(0, 0, W, H);
  if (active.length && !userCam) {
    let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
    for (const n of active) {
      x0 = Math.min(x0, n.x); x1 = Math.max(x1, n.x);
      y0 = Math.min(y0, n.y); y1 = Math.max(y1, n.y);
    }
    const fit = Math.min(2, (W - PANEL) / (x1 - x0 + 220), H / (y1 - y0 + 220));
    view.k += (fit - view.k) * 0.05;
    view.x += ((x0 + x1) / 2 - view.x) * 0.07;
    view.y += ((y0 + y1) / 2 - view.y) * 0.07;
  }
  ctx.translate((W + PANEL) / 2, H / 2); ctx.scale(view.k, view.k); ctx.translate(-view.x, -view.y);

  // A filter holds the lit set until it is cleared; otherwise hovering or clicking a
  // node lights it and its neighbours. Both fade everything else the same way.
  const focus = selected || hover;
  let lit = null;
  if (filter) {
    lit = new Set(active.filter(matchesFilter).map(n => n.i));
  } else if (focus) {
    lit = new Set([focus.i]);
    for (const j of neighbours[focus.i]) if (born.has(j)) lit.add(j);
  }

  ctx.lineWidth = 0.6 / view.k;
  for (const l of activeLinks) {
    const on = lit && lit.has(l.source.i) && lit.has(l.target.i);
    ctx.strokeStyle = lit ? (on ? theme.linkLit : theme.linkDim) : theme.link;
    ctx.beginPath(); ctx.moveTo(l.source.x, l.source.y); ctx.lineTo(l.target.x, l.target.y); ctx.stroke();
  }
  for (const n of active) {
    const st = STYLE[n.g], colour = groupColor(n.g), dim = lit && !lit.has(n.i);
    ctx.globalAlpha = dim ? 0.12 : 1;
    if (st.glow && !dim) {
      const halo = ctx.createRadialGradient(n.x, n.y, n.r, n.x, n.y, n.r + st.glow / 3);
      halo.addColorStop(0, colour + '55'); halo.addColorStop(1, colour + '00');
      ctx.fillStyle = halo;
      ctx.beginPath(); ctx.arc(n.x, n.y, n.r + st.glow / 3, 0, 7); ctx.fill();
    }
    ctx.fillStyle = colour;
    ctx.beginPath(); ctx.arc(n.x, n.y, n.r, 0, 7); ctx.fill();
  }
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  for (const n of active) {
    const icon = nodeIcon(n);
    if (!icon || view.k * n.r < 7) continue;   // below this the glyph is unreadable mush
    ctx.globalAlpha = lit && !lit.has(n.i) ? 0.12 : 1;
    ctx.font = `${n.r * 1.35}px ${EMOJI_FONT}`;
    ctx.fillText(icon, n.x, n.y);
  }
  ctx.textBaseline = 'alphabetic';
  ctx.globalAlpha = 1;
  ctx.font = `500 ${LABEL_PX / view.k}px ${LABEL_FONT}`;
  ctx.fillStyle = theme.label;
  for (const n of active) {
    if (lit ? lit.has(n.i) : view.k * n.r > 8) {
      ctx.fillText(n.label, n.x, n.y - n.r - 5 / view.k);
    }
  }
  document.getElementById('count').textContent = filter
    ? `${lit.size} of ${active.length} notes · ${filter.key}`
    : `${active.length} notes · ${activeLinks.length} links`;
}

function frame(now) {
  clock += now - last; last = now;
  if (!done) { activate(clock); if (nextIdx >= nodes.length) done = true; }
  if (alpha > ALPHA_FLOOR) tick();
  draw();
  requestAnimationFrame(frame);
}

