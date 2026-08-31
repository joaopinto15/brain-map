const GRAPH = __GRAPH__;
const TOKEN = '__TOKEN__';
const STYLE = GRAPH.groups;

// A theme paints the page chrome, the canvas, and — when it brings a palette — the
// group colours too. Add an entry here and it shows up in the picker.
const THEMES = {
  midnight: {
    name: 'Midnight',
    bg: '#07090d', panel: 'rgba(10,13,18,.85)', border: '#1f2937',
    text: '#e5e7eb', muted: '#9ca3af', accent: '#34d399',
    inputBg: '#0d1320', inputBorder: '#283548', btnBg: '#111827', btnHover: '#1c2a3f',
    heading: '#f1f5f9', strong: '#fde68a', code: '#f9a8d4',
    canvasBg: '#07090d', link: 'rgba(90,110,140,.28)', linkLit: 'rgba(120,160,210,.7)',
    linkDim: 'rgba(90,110,140,.05)', label: 'rgba(229,231,235,.85)',
    palette: null, router: null,
  },
  onedark: {
    name: 'One Dark',
    bg: '#282c34', panel: 'rgba(33,37,43,.9)', border: '#3e4451',
    text: '#abb2bf', muted: '#5c6370', accent: '#98c379',
    inputBg: '#21252b', inputBorder: '#3e4451', btnBg: '#2c313a', btnHover: '#3e4451',
    heading: '#d7dae0', strong: '#e5c07b', code: '#e06c75',
    canvasBg: '#282c34', link: 'rgba(92,99,112,.4)', linkLit: 'rgba(97,175,239,.75)',
    linkDim: 'rgba(92,99,112,.08)', label: 'rgba(171,178,191,.9)',
    palette: ['#61afef', '#e5c07b', '#c678dd', '#98c379', '#e06c75', '#56b6c2',
              '#d19a66', '#be5046', '#7f848e', '#abb2bf', '#528bff', '#c8ae9d'],
    router: '#98c379',
  },
};
// Chrome first, then the three the note renderer uses: headings, bold, and code.
const CSS_KEYS = ['bg', 'panel', 'border', 'text', 'muted', 'accent',
                  'inputBg', 'inputBorder', 'btnBg', 'btnHover',
                  'heading', 'strong', 'code'];
let theme = THEMES.midnight;

function applyTheme(key) {
  theme = THEMES[key] || THEMES.midnight;
  const root = document.documentElement;
  for (const k of CSS_KEYS) {
    root.style.setProperty('--' + k.replace(/[A-Z]/g, m => '-' + m.toLowerCase()), theme[k]);
  }
  try { localStorage.setItem('brain-map-theme', key); } catch {}
  drawLegend();
}

// Themes without a palette keep the colours the vault itself decided.
function groupColor(key) {
  if (!theme.palette) return STYLE[key].c;
  if (key === 'router') return theme.router;
  if (key === 'external') return theme.muted;
  const order = Object.keys(STYLE).filter(k => k !== 'router' && k !== 'external');
  return theme.palette[Math.max(0, order.indexOf(key)) % theme.palette.length];
}
// Every emoji comes off the wire: `icons.rs` resolves each tag and group against
// the keyword table vendored from Omarchy, so the page holds no emoji of its own.
const ICONS = GRAPH.icons || {};
const EMOJI_FONT = '"Apple Color Emoji", "Noto Color Emoji", "Segoe UI Emoji", sans-serif';
const tagIcon = tag => ICONS[tag] || '';
const nodeIcon = n => tagIcon((n.tags || [])[0]) || tagIcon(n.g);

