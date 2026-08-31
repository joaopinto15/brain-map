// A small markdown renderer — the page takes no dependencies, so this covers the
// blocks notes actually use and leaves the rest as text.
// ponytail: no reference links, no setext headings, no inline HTML — HTML stays escaped
// on purpose, a note is not trusted markup. Reach for a parser if notes outgrow this.
const escHtml = s => s.replace(/[&<>]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' }[c]));
const attr = s => s.replace(/"/g, '&quot;');
const unesc = s => s.replace(/&(amp|lt|gt|quot);/g, (_, e) =>
  ({ amp: '&', lt: '<', gt: '>', quot: '"' }[e]));
let footnotes = new Map();
function inline(s) {
  // Code spans come out first and go back in last, so nothing rewrites their contents.
  const code = [];
  const held = escHtml(s).replace(/`([^`]+)`/g, (_, c) => `\u0000${code.push(c) - 1}\u0000`);
  return held
    .replace(/!\[([^\]]*)\]\((https?:[^)\s]+)\)/g, '<img src="$2" alt="$1" loading="lazy">')
    .replace(/!?\[\[([^\]]+)\]\]/g, (_, t) => {
      const [target, alias] = [t.split('|')[0], t.split('|').pop()];
      return `<a class="wiki" data-note="${attr(target)}">${alias.split('#')[0]}</a>`;
    })
    .replace(/\[\^([^\]]+)\]/g, (m, id) => {
      const fn = footnotes.get(unesc(id));
      return fn ? `<sup class="fn" title="${attr(fn.text)}">${fn.n}</sup>` : m;
    })
    .replace(/\[([^\]]*)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
    .replace(/\[([^\]]*)\]\(([^)\s]*)\)/g, (_, label, target) =>
      `<a class="wiki" data-note="${attr(target)}" data-rel>${label}</a>`)
    .replace(/&lt;(https?:[^&\s]+)&gt;/g, '<a href="$1" target="_blank" rel="noopener">$1</a>')
    .replace(/~~([^~]+)~~/g, '<del>$1</del>')
    .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
    .replace(/(^|[^*])\*([^*]+)\*/g, '$1<i>$2</i>')
    .replace(/\u0000(\d+)\u0000/g, (_, i) => `<code>${code[i]}</code>`);
}
const cells = l => l.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(c => c.trim());
const isRule = l => /^\s*\|?[\s:|-]*-[\s:|-]*\|?\s*$/.test(l) && l.includes('-');
const alignOf = d => d.startsWith(':') && d.endsWith(':') ? 'center'
  : d.endsWith(':') ? 'right' : d.startsWith(':') ? 'left' : '';
const cell = (tag, html, align) =>
  `<${tag}${align ? ` style="text-align:${align}"` : ''}>${html}</${tag}>`;

function markdown(text) {
  const body = text.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, '').replace(/<!--[\s\S]*?-->/g, '');
  const lines = body.split('\n').map(l => l.replace(/\s+$/, '').replace(/\t/g, '  '));

  // Footnote definitions are collected first so a reference can carry its text,
  // whichever order they appear in.
  footnotes = new Map();
  const defs = [];
  for (const line of lines) {
    const def = line.match(/^\[\^([^\]]+)\]:\s*(.*)/);
    if (def) {
      footnotes.set(def[1], { n: defs.length + 1, text: def[2] });
      defs.push(def);
    }
  }

  const out = [];
  const lists = [];
  let quote = false, fence = null, lang = '', para = [];
  // Consecutive lines are one paragraph, the way markdown means them.
  const flush = () => { if (para.length) { out.push(`<p>${para.join(' ')}</p>`); para = []; } };
  const closeLists = (indent = -1) => {
    while (lists.length && lists[lists.length - 1].indent > indent) {
      const done = lists.pop();
      out.push(done.nested ? `</${done.tag}></li>` : `</${done.tag}>`);
    }
  };
  const close = () => {
    flush();
    closeLists();
    if (quote) { out.push('</blockquote>'); quote = false; }
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (fence !== null) {
      if (/^\s*```/.test(line)) {
        out.push(`<pre><code${lang ? ` class="language-${lang}"` : ''}>${escHtml(fence)}</code></pre>`);
        fence = null;
      } else fence += (fence ? '\n' : '') + line;
      continue;
    }

    const opening = line.match(/^\s*```\s*([\w+-]*)/);
    if (opening) { close(); fence = ''; lang = opening[1]; continue; }
    // A blank line ends a quote and a paragraph, but not a list: a loose list keeps going.
    if (!line.trim()) {
      flush();
      if (quote) { out.push('</blockquote>'); quote = false; }
      continue;
    }
    if (/^\[\^([^\]]+)\]:/.test(line)) continue;

    const heading = line.match(/^(#{1,6})\s+(.*)/);
    const item = line.match(/^(\s*)([-*+]|\d+[.)])\s+(.*)/);
    const quoted = line.match(/^>\s?(.*)/);
    // A table is a row whose next line is the ---|--- rule under it.
    const table = line.includes('|') && isRule(lines[i + 1] || '');

    if (table) {
      close();
      const head = cells(line), align = cells(lines[i + 1]).map(alignOf);
      out.push('<div class="scroll"><table><thead><tr>' +
        head.map((h, j) => cell('th', inline(h), align[j])).join('') + '</tr></thead><tbody>');
      for (i += 2; i < lines.length && lines[i].includes('|') && lines[i].trim(); i++) {
        const row = cells(lines[i]);
        out.push('<tr>' + head.map((_, j) => cell('td', inline(row[j] ?? ''), align[j])).join('') + '</tr>');
      }
      i--;
      out.push('</tbody></table></div>');
    } else if (heading) {
      close();
      out.push(`<h${heading[1].length}>${inline(heading[2])}</h${heading[1].length}>`);
    } else if (/^(---+|\*\*\*+|___+)$/.test(line.trim())) {
      close(); out.push('<hr>');
    } else if (item) {
      const indent = item[1].length;
      const want = /^\d/.test(item[2]) ? 'ol' : 'ul';
      flush();
      if (quote) { out.push('</blockquote>'); quote = false; }
      closeLists(indent);
      const open = lists[lists.length - 1];
      if (!open || indent > open.indent) {
        // Reopen the <li> above so the sublist nests inside it rather than beside it.
        if (open) out[out.length - 1] = out[out.length - 1].replace(/<\/li>$/, '');
        out.push(`<${want}>`);
        lists.push({ tag: want, indent, nested: !!open });
      }
      else if (open.tag !== want) {
        out.push(`</${lists.pop().tag}><${want}>`);
        lists.push({ tag: want, indent });
      }
      const task = item[3].match(/^\[([ xX])\]\s+(.*)/);
      out.push(`<li>${task ? (task[1] === ' ' ? '☐ ' : '☑ ') + inline(task[2]) : inline(item[3])}</li>`);
    } else if (quoted) {
      if (!quote) { flush(); closeLists(); out.push('<blockquote>'); quote = true; }
      para.push(inline(quoted[1]));
    } else if (lists.length && /^\s/.test(line) && /<\/li>$/.test(out[out.length - 1] || '')) {
      // A wrapped continuation line belongs to the item above it.
      out[out.length - 1] = out[out.length - 1].replace(/<\/li>$/, ` ${inline(line.trim())}</li>`);
    } else {
      if (!quote) closeLists();
      para.push(inline(line));
    }
  }
  if (fence !== null) out.push(`<pre><code>${escHtml(fence)}</code></pre>`);
  close();

  if (defs.length) {
    out.push('<hr><ol class="fn">');
    for (const [, id, text] of defs) out.push(`<li id="fn-${attr(id)}">${inline(text)}</li>`);
    out.push('</ol>');
  }
  return out.join('');
}

